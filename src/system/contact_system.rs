use amethyst::core::math::{Point3, Vector2, Vector3};
use amethyst::core::{Time, Transform};
use amethyst::ecs::{
    Entities, Entity, Join, Read, ReadExpect, ReadStorage, System, Write, WriteStorage,
};
use amethyst::shrev::{EventChannel, ReaderId};
use amethyst_extra::nphysics_ecs::*;
use amethyst_extra::BhopMovement3D;
use hoppinworld_runtime::{CustomStateEvent, ObjectType, PlayerTag, RuntimeProgress};
use RelativeTimer;

/// Very game dependent.
/// Don't try to make that generic.
#[derive(Default)]
pub struct ContactSystem {
}

impl<'a> System<'a> for ContactSystem {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, Transform>,
        ReadExpect<'a, GeometricalWorldRes<f32>>,
        Write<'a, RelativeTimer>,
        Read<'a, Time>,
        ReadStorage<'a, ObjectType>,
        ReadStorage<'a, PlayerTag>,
        ReadStorage<'a, BhopMovement3D>,
        WriteRigidBodies<'a, f32>,
        Write<'a, EventChannel<CustomStateEvent>>,
        Write<'a, RuntimeProgress>,
    );

    fn run(
        &mut self,
        (
            entities,
            mut transforms,
            geo_world,
            mut timer,
            time,
            object_types,
            players,
            bhop_movements,
            mut rigid_bodies,
            mut state_eventchannel,
            mut runtime_progress,
        ): Self::SystemData,
    ) {
        // Contact events
        /*for contact in contacts2.read(&mut self.collision_reader.as_mut().unwrap()) {
            info!("coll collision");
        }*/

        // Proximity events
        for contact in geo_world.proximity_events() {
            info!("Collision: {:?}", contact);
            let type1 = object_types.get(contact.collider1);
            let type2 = object_types.get(contact.collider2);

            if type1.is_none() || type2.is_none() {
                continue;
            }
            let type1 = type1.unwrap();
            let type2 = type2.unwrap();

            let (_player, other, player_entity) = if *type1 == ObjectType::Player {
                //(contact.bodies.0,contact.bodies.1)
                (type1, type2, contact.collider1)
            } else if *type2 == ObjectType::Player {
                //(contact.bodies.1,contact.bodies.0)
                (type2, type1, contact.collider2)
            } else {
                continue;
            };

            match other {
                ObjectType::StartZone => {
                    if runtime_progress.current_segment == 1 {
                        timer.start(time.absolute_time_seconds());
                    }
                    // Also limit player velocity while touching the StartZone to prevent any early starts.
                    // Not sure if this should go into state or not. Since it is heavily related to gameplay I'll put it here.
                    for (entity, _, movement, mut rb) in
                        (&*entities, &players, &bhop_movements, &mut rigid_bodies).join()
                    {
                        if entity == player_entity {
                            let max_vel = movement.max_velocity_ground;
                            let cur_vel3 = rb.velocity().linear;
                            let mut cur_vel_flat = Vector2::new(cur_vel3.x, cur_vel3.z);
                            let cur_vel_flat_mag = cur_vel_flat.magnitude();
                            if cur_vel_flat_mag >= max_vel {
                                cur_vel_flat = cur_vel_flat.normalize() * max_vel;
                                rb.set_linear_velocity(Vector3::new(cur_vel_flat.x, cur_vel3.y, cur_vel_flat.y));
                            }
                        }
                    }

                    info!("start zone!");
                }
                ObjectType::EndZone => {
                    timer.stop();
                    info!("Finished! time: {:?}", timer.duration());
                    let id = runtime_progress.segment_count as usize;
                    runtime_progress.segment_times[id - 1] = timer.duration();
                    state_eventchannel.single_write(CustomStateEvent::MapFinished);
                }
                ObjectType::KillZone => {
                    warn!("you are ded!");
                    let seg = runtime_progress.current_segment;
                    let pos = if seg == 1 {
                        // To start zone
                        (&transforms, &object_types)
                            .join()
                            .filter(|(_, obj)| **obj == ObjectType::StartZone)
                            .map(|(tr, _)| tr.translation())
                            .next()
                            .unwrap()
                            .clone()
                    } else {
                        // To last checkpoint

                        // Find checkpoint corresponding to the current segment in progress
                        (&transforms, &object_types)
                            .join()
                            .filter(|(_, obj)| {
                                if let ObjectType::SegmentZone(s) = **obj {
                                    s == seg - 1
                                } else {
                                    false
                                }
                            })
                            .map(|(tr, _)| tr.translation())
                            .next()
                            .unwrap()
                            .clone()
                    };

                    // Move the player
                    (&players, &mut transforms).join().for_each(|(_, tr)| {
                        *tr.translation_mut() = pos;
                    });
                }
                ObjectType::SegmentZone(id) => {
                    if *id >= runtime_progress.current_segment {
                        runtime_progress.segment_times[(id - 1) as usize] = timer.duration();
                        runtime_progress.current_segment = id + 1;
                    }
                    info!("segment done");
                }
                _ => {}
            }
        }
    }
}
