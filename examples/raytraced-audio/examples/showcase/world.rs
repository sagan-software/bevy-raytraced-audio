//! Modular house plans, matching acoustic proxies, terrain and operable openings.

use super::controller::{Player, View};
use bevy::prelude::*;
use bevy_raytraced_audio::{AcousticMaterial, BandAbsorption, BandGain};
use bevy_raytraced_audio_3d::RaytracedAudioSurface3d;

/// Story spacing matches the 2.5 m Kenney staircase.
pub(super) const STORY: f32 = 2.5;
/// Bridge lies in the front gardens, clear of both houses' side walls.
pub(super) const BRIDGE_Z: f32 = 5.0;
/// Locations of the speech cottage and the three-story footstep house.
pub(super) const HOMES: [f32; 2] = [-10.0, 10.0];

/// Acoustic material presets used by explicit, lightweight geometry.
#[derive(Clone, Copy)]
pub(super) enum Surface {
    /// Masonry/plaster shell.
    Wall,
    /// Timber floors and doors.
    Wood,
    /// Thin window panes.
    Glass,
    /// Absorbent ground.
    Soil,
    /// Hard, lightly absorbing room floor.
    Tile,
    /// Carpet over a structural floor, absorbing mostly mids and highs.
    Carpet,
    /// Upholstered cushions and mattresses.
    Fabric,
    /// Books and irregular shelving.
    Books,
}
impl Surface {
    /// Produces bounded absorption and transmission fractions.
    pub(super) fn material(self) -> AcousticMaterial {
        let (absorption, transmission, scattering) = match self {
            Self::Wall => ([0.12, 0.10, 0.08], [0.28, 0.08, 0.015], 0.12),
            Self::Wood => ([0.18, 0.15, 0.12], [0.45, 0.18, 0.045], 0.2),
            Self::Glass => ([0.05, 0.04, 0.03], [0.65, 0.30, 0.09], 0.05),
            Self::Soil => ([0.65, 0.8, 0.9], [0.0, 0.0, 0.0], 0.6),
            Self::Tile => ([0.02, 0.03, 0.04], [0.45, 0.18, 0.045], 0.05),
            Self::Carpet => ([0.08, 0.45, 0.75], [0.45, 0.18, 0.045], 0.65),
            Self::Fabric => ([0.25, 0.7, 0.9], [0.25, 0.08, 0.015], 0.85),
            Self::Books => ([0.12, 0.35, 0.55], [0.2, 0.06, 0.015], 0.8),
        };
        AcousticMaterial::new(
            BandAbsorption::try_new(absorption[0], absorption[1], absorption[2])
                .expect("bounded absorption"),
        )
        .try_with_transmission(
            BandGain::try_new(transmission[0], transmission[1], transmission[2])
                .expect("bounded transmission"),
        )
        .expect("energy budget")
        .try_with_scattering(scattering)
        .expect("bounded scattering")
    }
}

/// Repeatable room treatment comparison: geometry and ears stay in the same place.
#[derive(Resource, Default)]
pub(super) struct RoomTreatment {
    /// 0 = empty/tile, 1 = empty/carpet, 2 = carpet/furnished.
    pub(super) mode: usize,
}
impl RoomTreatment {
    /// Short label used by the listening guide.
    pub(super) const fn label(&self) -> &'static str {
        match self.mode {
            0 => "empty / tile",
            1 => "empty / carpet",
            _ => "carpet / furnished",
        }
    }
}

/// A floor's triangle is retained so its acoustic material can be changed in place.
#[derive(Component)]
pub(super) struct RoomFloor([Vec3; 3]);

/// A furniture face can be removed from the acoustic scene and restored exactly.
#[derive(Component)]
pub(super) struct FurnitureFace(RaytracedAudioSurface3d);

/// Controls furnishings and floor overlays alongside the normal story cutaway.
#[derive(Component)]
pub(super) enum TreatmentVisual {
    /// Only visible in the furnished mode.
    Furniture,
    /// Tile or carpet overlay, selected by whether the floor is soft.
    Floor(bool),
}

/// An oriented box shared by body collision and its central acoustic plane.
#[derive(Component, Clone, Copy)]
pub(super) struct Solid {
    /// Local center, relative to its entity transform.
    pub(super) center: Vec3,
    /// Local half dimensions.
    pub(super) half: Vec3,
}

/// Render asset and acoustic boxes comprising one stationary module.
#[derive(Clone)]
pub(super) struct Module {
    /// Path relative to assets/models.
    pub(super) model: &'static str,
    /// Placement in world coordinates.
    pub(super) pose: Transform,
    /// Wall frame or floor pieces, in model coordinates.
    pub(super) boxes: Vec<Solid>,
    /// Shared acoustic preset.
    pub(super) surface: Surface,
    /// Whether the boxes block the character horizontally.
    pub(super) blocks: bool,
    /// Lowest story to display this module in overhead cutaway view.
    pub(super) level: f32,
}

/// A floor or ramp on which the player's feet can rest.
#[derive(Clone, Copy)]
pub(super) struct WalkSurface {
    /// Horizontal center.
    pub(super) center: Vec2,
    /// Horizontal half dimensions.
    pub(super) half: Vec2,
    /// Height at the center.
    pub(super) height: f32,
    /// Bottom of a solid staircase; `None` allows walking below bridges and floors.
    pub(super) solid_from: Option<f32>,
    /// Height increase per world X/Z meter.
    pub(super) slope: Vec2,
}
impl WalkSurface {
    /// Prevents entry into the solid volume below a staircase, but not into lower stories.
    pub(super) fn blocks(self, feet: Vec3) -> bool {
        self.solid_from.is_some_and(|bottom| feet.y + 1.6 > bottom)
            && self.at(feet.xz()).is_some_and(|top| feet.y + 0.34 < top)
    }

    /// Evaluates the support height inside its footprint.
    pub(super) fn at(self, position: Vec2) -> Option<f32> {
        let delta = position - self.center;
        (delta.abs().cmple(self.half).all()).then_some(self.height + delta.dot(self.slope))
    }
}

/// One hinge-controlled door or window leaf.
#[derive(Component)]
pub(super) struct Gate {
    /// Shared identifier lets double leaves operate together.
    pub(super) group: usize,
    /// User-facing description.
    pub(super) label: &'static str,
    /// Closed hinge pose.
    pub(super) closed: Transform,
    /// Requested open state.
    pub(super) open: bool,
    /// Current signed rotation about local Y.
    pub(super) angle: f32,
    /// Rotation reached when open.
    pub(super) open_angle: f32,
    /// Local point used for interaction distance.
    pub(super) handle: Vec3,
}

/// Hides upper stories visually without altering their acoustic geometry.
#[derive(Component)]
pub(super) struct Cutaway(pub(super) f32);

/// A reversible wall section at a fixed, meaningful location in the house shell.
pub(super) struct BuildSite {
    /// Full module to restore.
    pub(super) module: Module,
    /// Current root, absent while destroyed.
    pub(super) entity: Option<Entity>,
}

/// Shared plan for movement and reversible modifications.
#[derive(Resource, Default)]
pub(super) struct Village {
    /// All traversable floor and ramp footprints.
    pub(super) supports: Vec<WalkSurface>,
    /// Dedicated editable wall sections.
    pub(super) sites: Vec<BuildSite>,
    /// Contextual action shown in the HUD.
    pub(super) prompt: String,
}

/// Central plane of a thin box, used consistently by tests and the live scene.
pub(super) fn plane(solid: Solid) -> [Vec3; 4] {
    let h = solid.half;
    let (a, b) = if h.x <= h.y && h.x <= h.z {
        (Vec3::Y * h.y, Vec3::Z * h.z)
    } else if h.y <= h.z {
        (Vec3::X * h.x, Vec3::Z * h.z)
    } else {
        (Vec3::X * h.x, Vec3::Y * h.y)
    };
    [
        solid.center - a - b,
        solid.center + a - b,
        solid.center + a + b,
        solid.center - a + b,
    ]
}

/// Adds two triangles for a rectangular surface, optionally sharing a collider.
fn acoustic_box(
    commands: &mut Commands<'_, '_>,
    parent: Entity,
    solid: Solid,
    material: Surface,
    blocks: bool,
) {
    let [a, b, c, d] = plane(solid);
    for points in [[a, b, c], [a, c, d]] {
        commands.spawn((
            RaytracedAudioSurface3d::new(points, material.material()).expect("rectangle triangle"),
            Transform::default(),
            ChildOf(parent),
        ));
    }
    if blocks {
        commands.spawn((solid, Transform::default(), ChildOf(parent)));
    }
}

/// Loads an actual modular GLB, independent of its small acoustic proxy.
fn model(
    commands: &mut Commands<'_, '_>,
    server: &AssetServer,
    path: &str,
    pose: Transform,
) -> Entity {
    commands
        .spawn((
            WorldAssetRoot(server.load(format!("models/{path}.glb#Scene0"))),
            pose,
        ))
        .id()
}

/// Spawns a stationary module and matching geometry from the common plan.
pub(super) fn spawn_module(
    commands: &mut Commands<'_, '_>,
    server: &AssetServer,
    module: &Module,
) -> Entity {
    let root = model(commands, server, module.model, module.pose);
    commands.entity(root).insert(Cutaway(module.level));
    for solid in &module.boxes {
        let floor = module.model == "building/floor"
            && module.level
                < if module.pose.translation.x < 0.0 {
                    STORY
                } else {
                    STORY * 2.0
                };
        if floor {
            let [a, b, c, d] = plane(*solid);
            for points in [[a, b, c], [a, c, d]] {
                commands.spawn((
                    RoomFloor(points),
                    RaytracedAudioSurface3d::new(points, Surface::Carpet.material())
                        .expect("floor"),
                    Transform::default(),
                    ChildOf(root),
                ));
            }
        } else {
            acoustic_box(commands, root, *solid, module.surface, module.blocks);
        }
    }
    root
}

/// Wall piece with its center and dimensions in the kit's YZ plane.
fn wall_box(y: f32, z: f32, height: f32, width: f32) -> Solid {
    Solid {
        center: Vec3::new(0.0, y, z),
        half: Vec3::new(0.10, height / 2.0, width / 2.0),
    }
}

/// Physical opening shapes follow the mesh's measured inner dimensions.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Opening {
    /// Full wall.
    None,
    /// 0.9 m door.
    Door,
    /// 2.9 m double doorway.
    Double,
    /// 0.9 by 1.2 m window.
    Window,
}

/// Produces a kit wall and only the solid portions surrounding its aperture.
pub(super) fn wall(pose: Transform, opening: Opening) -> Module {
    let (asset, width, aperture, bottom, top) = match opening {
        Opening::None => ("building/wall", 2.0, 0.0, 0.0, 0.0),
        Opening::Door => ("building/wall-doorway-square", 2.0, 0.9, 0.0, 2.1),
        Opening::Double => ("building/wall-doorway-wide-square", 4.0, 2.9, 0.0, 2.1),
        Opening::Window => ("building/wall-window-open", 2.0, 0.9, 0.7, 1.9),
    };
    let boxes = if opening == Opening::None {
        vec![wall_box(1.25, 0.0, 2.5, width)]
    } else {
        let side = (width - aperture) / 2.0;
        let mut parts = vec![
            wall_box(1.25, -f32::midpoint(aperture, side), 2.5, side),
            wall_box(1.25, f32::midpoint(aperture, side), 2.5, side),
            wall_box(f32::midpoint(top, 2.5), 0.0, 2.5 - top, aperture),
        ];
        if bottom > 0.0 {
            parts.push(wall_box(bottom / 2.0, 0.0, bottom, aperture));
        }
        parts
    };
    Module {
        model: asset,
        level: pose.translation.y,
        pose,
        boxes,
        surface: Surface::Wall,
        blocks: true,
    }
}

/// Returns a tiled house shell with apertures, upper stories and genuine stairwell holes.
#[expect(
    clippy::float_cmp,
    reason = "exact integer and half-integer module coordinates identify discrete tiles"
)]
pub(super) fn house_plan(center: f32, vertical: bool) -> (Vec<Module>, Vec<WalkSurface>) {
    let mut modules = Vec::new();
    let mut supports = Vec::new();
    let levels: &[f32] = if vertical {
        &[-STORY, 0.0, STORY]
    } else {
        &[0.0]
    };
    for &y in levels {
        for side in [-1.0_f32, 1.0] {
            for offset in [-3.0, -1.0, 1.0, 3.0] {
                let opening = if y >= 0.0 && offset == -1.0 {
                    Opening::Window
                } else {
                    Opening::None
                };
                modules.push(wall(
                    Transform::from_xyz(side.mul_add(4.0, center), y, offset - 2.0),
                    opening,
                ));
                let opening = if y == 0.0 && side > 0.0 && offset == 1.0 {
                    Opening::Door
                } else if y >= 0.0 && offset == -1.0 {
                    Opening::Window
                } else {
                    Opening::None
                };
                modules.push(wall(
                    Transform::from_xyz(center + offset, y, side.mul_add(4.0, -2.0))
                        .with_rotation(Quat::from_rotation_y(core::f32::consts::FRAC_PI_2)),
                    opening,
                ));
            }
        }
        for x in [-3.0, -1.0, 1.0, 3.0] {
            for z in [-5.0, -3.0, -1.0, 1.0] {
                let hole = vertical
                    && ((y == 0.0 && x == -3.0) || (y == STORY && x == 3.0))
                    && (-3.0..=-1.0).contains(&z);
                if !hole {
                    add_floor(&mut modules, &mut supports, Vec3::new(center + x, y, z));
                }
            }
        }
    }
    let roof_y = if vertical { STORY * 2.0 } else { STORY };
    for x in [-3.0, -1.0, 1.0, 3.0] {
        for z in [-5.0, -3.0, -1.0, 1.0] {
            add_floor(
                &mut modules,
                &mut Vec::new(),
                Vec3::new(center + x, roof_y, z),
            );
        }
    }
    (modules, supports)
}

/// Adds a 2 m kit floor whose top is the walking height.
fn add_floor(modules: &mut Vec<Module>, supports: &mut Vec<WalkSurface>, top: Vec3) {
    modules.push(Module {
        model: "building/floor",
        pose: Transform::from_translation(top - Vec3::Y * 0.1),
        boxes: vec![Solid {
            center: Vec3::Y * 0.05,
            half: Vec3::new(1.0, 0.05, 1.0),
        }],
        surface: Surface::Wood,
        blocks: false,
        level: top.y,
    });
    supports.push(WalkSurface {
        center: top.xz(),
        half: Vec2::ONE,
        height: top.y,
        slope: Vec2::ZERO,
        solid_from: None,
    });
}

/// Installs house shells, gates, terrain and furnished interiors.
#[expect(
    clippy::float_cmp,
    reason = "selecting exact authored module grid coordinates"
)]
pub(super) fn setup(
    mut commands: Commands<'_, '_>,
    server: Res<'_, AssetServer>,
    mut meshes: ResMut<'_, Assets<Mesh>>,
    mut materials: ResMut<'_, Assets<StandardMaterial>>,
    mut village: ResMut<'_, Village>,
) {
    commands.spawn((
        DirectionalLight {
            illuminance: 9_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(-12.0, 20.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    for (index, center) in HOMES.into_iter().enumerate() {
        let (modules, supports) = house_plan(center, index == 1);
        village.supports.extend(supports);
        for module in modules {
            let editable = module.model == "building/wall"
                && module.pose.translation.y == 0.0
                && (module.pose.translation.x - (center - 4.0)).abs() < 0.01
                && module.pose.translation.z == 1.0;
            let entity = spawn_module(&mut commands, &server, &module);
            if module.model == "building/floor"
                && module.level < if index == 0 { STORY } else { STORY * 2.0 }
            {
                for soft in [false, true] {
                    let overlay = cuboid(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        module.pose.translation + Vec3::Y * 0.105,
                        Vec3::new(1.99, 0.012, 1.99),
                        if soft {
                            Color::srgb(0.35, 0.31, 0.25)
                        } else {
                            Color::srgb(0.68, 0.70, 0.68)
                        },
                    );
                    commands
                        .entity(overlay)
                        .insert((Cutaway(module.level), TreatmentVisual::Floor(soft)));
                }
            }
            if editable {
                village.sites.push(BuildSite {
                    module,
                    entity: Some(entity),
                });
            }
        }
        install_gates(&mut commands, &server, center, index);
        furnish(&mut commands, &server, center, index == 1);
    }
    stairs(&mut commands, &server, &mut village);
    terrain(
        &mut commands,
        &server,
        &mut meshes,
        &mut materials,
        &mut village,
    );
}

/// Places hinged leaves in measured doorway/window apertures.
fn spawn_gate(
    commands: &mut Commands<'_, '_>,
    server: &AssetServer,
    closed: Transform,
    opening: Opening,
    group: usize,
    reverse: bool,
) {
    let window = opening == Opening::Window;
    let width = if opening == Opening::Double {
        1.45
    } else {
        0.9
    };
    let height = if window { 1.2 } else { 2.1 };
    let path = if window {
        "building/window-leaf"
    } else {
        "building/door-rotate-square-a"
    };
    let root = commands
        .spawn((
            closed,
            Visibility::default(),
            Cutaway(closed.translation.y - if window { 0.7 } else { 0.0 }),
            Gate {
                group,
                label: if window {
                    "window"
                } else if opening == Opening::Double {
                    "double doors"
                } else {
                    "door"
                },
                closed,
                open: false,
                angle: 0.0,
                open_angle: if reverse { -1.5 } else { 1.5 },
                handle: Vec3::new(0.0, height / 2.0, width / 2.0),
            },
        ))
        .id();
    let visual = model(
        commands,
        server,
        path,
        Transform::from_scale(Vec3::new(1.0, 1.0, width / 0.9)),
    );
    commands.entity(visual).insert(ChildOf(root));
    acoustic_box(
        commands,
        root,
        Solid {
            center: Vec3::new(0.0, height / 2.0, width / 2.0),
            half: Vec3::new(0.055, height / 2.0, width / 2.0),
        },
        if window {
            Surface::Glass
        } else {
            Surface::Wood
        },
        true,
    );
}

/// Installs exterior openings and a broad internal double door in the speech cottage.
fn install_gates(commands: &mut Commands<'_, '_>, server: &AssetServer, center: f32, house: usize) {
    let mut id = house * 100;
    let (modules, _) = house_plan(center, house == 1);
    for module in modules {
        let opening = if module.model == "building/wall-window-open" {
            Opening::Window
        } else if module.model == "building/wall-doorway-square" {
            Opening::Door
        } else {
            continue;
        };
        let offset = Vec3::new(
            0.0,
            if opening == Opening::Window { 0.7 } else { 0.0 },
            -0.45,
        );
        let closed = Transform::from_translation(module.pose.transform_point(offset))
            .with_rotation(module.pose.rotation);
        spawn_gate(commands, server, closed, opening, id, false);
        id += 1;
    }
    if house == 0 {
        for z in [-5.0, 1.0] {
            spawn_module(
                commands,
                server,
                &wall(Transform::from_xyz(center, 0.0, z), Opening::None),
            );
        }
        spawn_module(
            commands,
            server,
            &wall(Transform::from_xyz(center, 0.0, -2.0), Opening::Double),
        );
        for reverse in [false, true] {
            let pose = Transform::from_xyz(center, 0.0, if reverse { -0.55 } else { -3.45 })
                .with_rotation(Quat::from_rotation_y(if reverse {
                    core::f32::consts::PI
                } else {
                    0.0
                }));
            spawn_gate(commands, server, pose, Opening::Double, 90, reverse);
        }
    }
}

/// Furniture identifies the rooms without substituting arbitrary geometry for the house kit.
fn furnish(commands: &mut Commands<'_, '_>, server: &AssetServer, x: f32, vertical: bool) {
    let objects = [
        ("loungeSofa", Vec3::new(-2.8, 0.0, -4.9), 0.0),
        ("tableCoffee", Vec3::new(-2.8, 0.0, -3.6), 0.0),
        ("rugRectangle", Vec3::new(-2.7, 0.005, -3.6), 0.0),
        ("bookcaseOpen", Vec3::new(2.8, 0.0, -5.6), 0.0),
        (
            "kitchenSink",
            Vec3::new(2.8, 0.0, -4.5),
            -core::f32::consts::FRAC_PI_2,
        ),
        (
            "kitchenStove",
            Vec3::new(2.8, 0.0, -3.5),
            -core::f32::consts::FRAC_PI_2,
        ),
        ("pottedPlant", Vec3::new(-3.4, 0.0, 1.4), 0.0),
    ];
    for (asset, position, yaw) in objects {
        // Keep the staircase landings and runners' lanes clear in the vertical house.
        if vertical && position.x > 0.0 {
            continue;
        }
        let root = model(
            commands,
            server,
            &format!("furniture/{asset}"),
            Transform::from_translation(position + Vec3::X * x)
                .with_rotation(Quat::from_rotation_y(yaw)),
        );
        commands
            .entity(root)
            .insert((Cutaway(0.0), TreatmentVisual::Furniture));
        furniture_acoustics(commands, root, asset);
    }
    if vertical {
        for (asset, position) in [
            ("bedDouble", Vec3::new(x - 2.0, STORY, -4.8)),
            ("bookcaseOpen", Vec3::new(x + 1.0, -STORY, -5.5)),
        ] {
            let root = model(
                commands,
                server,
                &format!("furniture/{asset}"),
                Transform::from_translation(position),
            );
            commands
                .entity(root)
                .insert((Cutaway(position.y), TreatmentVisual::Furniture));
            furniture_acoustics(commands, root, asset);
        }
    }
}

/// Approximate measured kit bounds, kept small enough to preserve door and staircase openings.
pub(super) fn furniture_bounds(asset: &str) -> (Vec3, Vec3, Surface) {
    match asset {
        "loungeSofa" => (
            Vec3::new(0.0, 0.0, -0.41),
            Vec3::new(0.98, 0.46, 0.0),
            Surface::Fabric,
        ),
        "tableCoffee" => (
            Vec3::new(-0.46, 0.0, -0.3),
            Vec3::new(0.2, 0.23, 0.1),
            Surface::Wood,
        ),
        "rugRectangle" => (
            Vec3::new(0.0, 0.0, -0.92),
            Vec3::new(1.57, 0.01, 0.0),
            Surface::Carpet,
        ),
        "bookcaseOpen" => (
            Vec3::new(0.0, 0.0, -0.25),
            Vec3::new(0.4, 0.88, 0.0),
            Surface::Books,
        ),
        "bedDouble" => (
            Vec3::new(0.0, 0.0, -1.05),
            Vec3::new(0.8, 0.4, 0.02),
            Surface::Fabric,
        ),
        "pottedPlant" => (
            Vec3::new(-0.08, 0.0, -0.45),
            Vec3::new(0.43, 1.08, 0.1),
            Surface::Soil,
        ),
        _ => (
            Vec3::new(0.0, 0.0, -0.45),
            Vec3::new(0.43, 0.49, 0.03),
            Surface::Wood,
        ),
    }
}

/// Six faces of a solid proxy; rays must encounter furniture from every direction.
pub(super) fn box_faces(min: Vec3, max: Vec3) -> [[Vec3; 4]; 6] {
    let p = |x, y, z| Vec3::new(x, y, z);
    [
        [
            p(min.x, min.y, min.z),
            p(min.x, max.y, min.z),
            p(min.x, max.y, max.z),
            p(min.x, min.y, max.z),
        ],
        [
            p(max.x, min.y, min.z),
            p(max.x, max.y, min.z),
            p(max.x, max.y, max.z),
            p(max.x, min.y, max.z),
        ],
        [
            p(min.x, min.y, min.z),
            p(max.x, min.y, min.z),
            p(max.x, min.y, max.z),
            p(min.x, min.y, max.z),
        ],
        [
            p(min.x, max.y, min.z),
            p(max.x, max.y, min.z),
            p(max.x, max.y, max.z),
            p(min.x, max.y, max.z),
        ],
        [
            p(min.x, min.y, min.z),
            p(max.x, min.y, min.z),
            p(max.x, max.y, min.z),
            p(min.x, max.y, min.z),
        ],
        [
            p(min.x, min.y, max.z),
            p(max.x, min.y, max.z),
            p(max.x, max.y, max.z),
            p(min.x, max.y, max.z),
        ],
    ]
}

/// Matches furniture visuals with lightweight three-dimensional acoustic proxies.
fn furniture_acoustics(commands: &mut Commands<'_, '_>, root: Entity, asset: &str) {
    let (min, max, material) = furniture_bounds(asset);
    for [a, b, c, d] in box_faces(min, max) {
        for points in [[a, b, c], [a, c, d]] {
            let surface =
                RaytracedAudioSurface3d::new(points, material.material()).expect("furniture face");
            commands.spawn((
                FurnitureFace(surface),
                surface,
                Transform::default(),
                ChildOf(root),
            ));
        }
    }
}

/// Changes physical materials and removes/restores furniture faces for controlled listening.
pub(super) fn room_treatment(
    treatment: Res<'_, RoomTreatment>,
    mut commands: Commands<'_, '_>,
    mut floors: Query<'_, '_, (&RoomFloor, &mut RaytracedAudioSurface3d)>,
    furniture: Query<'_, '_, (Entity, &FurnitureFace)>,
) {
    if !treatment.is_changed() {
        return;
    }
    let material = if treatment.mode == 0 {
        Surface::Tile
    } else {
        Surface::Carpet
    }
    .material();
    for (floor, mut surface) in &mut floors {
        *surface = RaytracedAudioSurface3d::new(floor.0, material).expect("floor");
    }
    for (entity, face) in &furniture {
        if treatment.mode == 2 {
            commands.entity(entity).insert(face.0);
        } else {
            commands.entity(entity).remove::<RaytracedAudioSurface3d>();
        }
    }
}

/// Shared staircase footprints used by rendering, movement and traversal regressions.
pub(super) fn stair_supports() -> [WalkSurface; 2] {
    [(7.0, -STORY), (13.0, 0.0)].map(|(x, y)| WalkSurface {
        center: Vec2::new(x, -2.0),
        half: Vec2::new(0.65, 2.0),
        height: y + 1.25,
        slope: Vec2::new(0.0, 0.625),
        solid_from: Some(y),
    })
}

/// Two stair flights use the kit's actual rising +Z direction, with holes in the floor above.
fn stairs(commands: &mut Commands<'_, '_>, server: &AssetServer, village: &mut Village) {
    for support in stair_supports() {
        let center_x = support.center.x;
        let base_y = support.height - 1.25;
        let root = model(
            commands,
            server,
            "building/stairs-closed",
            Transform::from_xyz(center_x, base_y, -2.0),
        );
        commands.entity(root).insert(Cutaway(base_y));
        let rails = model(
            commands,
            server,
            "building/stairs-sides",
            Transform::from_xyz(center_x, base_y, -2.0),
        );
        commands.entity(rails).insert(Cutaway(base_y));
        village.supports.push(support);
        let [a, b, c, d] = [
            Vec3::new(-0.65, 0.0, -2.0),
            Vec3::new(0.65, 0.0, -2.0),
            Vec3::new(0.65, 2.5, 2.0),
            Vec3::new(-0.65, 2.5, 2.0),
        ];
        for points in [[a, b, c], [a, c, d]] {
            commands.spawn((
                RaytracedAudioSurface3d::new(points, Surface::Wood.material())
                    .expect("stair triangle"),
                Transform::default(),
                ChildOf(root),
            ));
        }
    }
}

/// Creates a solid-colored primitive for terrain and small explanatory markers.
pub(super) fn cuboid(
    commands: &mut Commands<'_, '_>,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    center: Vec3,
    size: Vec3,
    color: Color,
) -> Entity {
    commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::from_size(size))),
            MeshMaterial3d(materials.add(color)),
            Transform::from_translation(center),
        ))
        .id()
}

/// Continuous bank profile used by both terrain rendering and walking support.
pub(super) fn ground_height(x: f32) -> f32 {
    ((x.abs() - 2.0) / 2.0).clamp(0.0, 1.0).mul_add(1.1, -1.1)
}

/// Builds banks, a lowered stream bed and an elevated modular wooden bridge.
fn terrain(
    commands: &mut Commands<'_, '_>,
    server: &AssetServer,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    village: &mut Village,
) {
    let grass = Color::srgb(0.34, 0.48, 0.28);
    // Tiled ground omits the basement footprint instead of sealing its stairs with terrain.
    for x in -11_i16..11 {
        for z in -9_i16..9 {
            let center = Vec3::new(
                f32::from(x).mul_add(2.0, 1.0),
                -0.15,
                f32::from(z).mul_add(2.0, 1.0),
            );
            if center.x.abs() < 4.0
                || (((6.0..14.0).contains(&center.x) || (-14.0..-6.0).contains(&center.x))
                    && (-6.0..2.0).contains(&center.z))
            {
                continue;
            }
            let root = cuboid(
                commands,
                meshes,
                materials,
                center,
                Vec3::new(2.0, 0.3, 2.0),
                grass,
            );
            acoustic_box(
                commands,
                root,
                Solid {
                    center: Vec3::Y * 0.15,
                    half: Vec3::new(1.0, 0.01, 1.0),
                },
                Surface::Soil,
                false,
            );
        }
    }
    cuboid(
        commands,
        meshes,
        materials,
        Vec3::new(0.0, -1.13, 0.0),
        Vec3::new(4.0, 0.1, 36.0),
        Color::srgb(0.20, 0.47, 0.52),
    );
    for side in [-1.0_f32, 1.0] {
        let root = cuboid(
            commands,
            meshes,
            materials,
            Vec3::new(side * 3.0, -0.55, 0.0),
            Vec3::new(2.283, 0.12, 36.0),
            grass,
        );
        commands.entity(root).insert(
            Transform::from_xyz(side * 3.0, -0.55, 0.0)
                .with_rotation(Quat::from_rotation_z(side * 0.5028)),
        );
    }
    // Center sections form a 6 m deck. Their original 1 m modules are scaled uniformly in X/Z.
    for x in [-2.0, 0.0, 2.0] {
        model(
            commands,
            server,
            "nature/bridge_center_wood",
            Transform::from_xyz(x, 1.0, BRIDGE_Z).with_scale(Vec3::splat(2.0)),
        );
    }
    let deck = commands
        .spawn(Transform::from_xyz(0.0, 1.15, BRIDGE_Z))
        .id();
    acoustic_box(
        commands,
        deck,
        Solid {
            center: Vec3::ZERO,
            half: Vec3::new(3.0, 0.08, 1.0),
        },
        Surface::Wood,
        false,
    );
    village.supports.push(WalkSurface {
        center: Vec2::new(0.0, BRIDGE_Z),
        half: Vec2::new(3.0, 0.85),
        height: 1.2,
        slope: Vec2::ZERO,
        solid_from: None,
    });
    for side in [-1.0_f32, 1.0] {
        let root = cuboid(
            commands,
            meshes,
            materials,
            Vec3::new(side * 4.5, 0.6, BRIDGE_Z),
            Vec3::new(3.231, 0.12, 2.0),
            Color::srgb(0.55, 0.36, 0.22),
        );
        commands.entity(root).insert(
            Transform::from_xyz(side * 4.5, 0.6, BRIDGE_Z)
                .with_rotation(Quat::from_rotation_z(-side * 0.3805)),
        );
        acoustic_box(
            commands,
            root,
            Solid {
                center: Vec3::ZERO,
                half: Vec3::new(1.6155, 0.06, 1.0),
            },
            Surface::Wood,
            false,
        );
        village.supports.push(WalkSurface {
            center: Vec2::new(side * 4.5, BRIDGE_Z),
            half: Vec2::new(1.5, 0.85),
            height: 0.6,
            slope: Vec2::new(-side * 0.4, 0.0),
            solid_from: None,
        });
    }
    for (x, z) in [
        (-18.0, -9.0),
        (-18.0, 6.0),
        (18.0, -9.0),
        (18.0, 7.0),
        (-8.0, 11.0),
        (8.0, 12.0),
    ] {
        model(
            commands,
            server,
            "nature/tree_oak",
            Transform::from_xyz(x, 0.0, z).with_scale(Vec3::splat(2.2)),
        );
    }
    for z in [-12.0, -7.0, 5.0, 10.0, 14.0] {
        for side in [-1.0_f32, 1.0] {
            model(
                commands,
                server,
                "nature/rock_largeA",
                Transform::from_xyz(side * 2.4, -0.95, z).with_scale(Vec3::splat(1.4)),
            );
        }
    }
}

/// Opens the nearest reachable gate; X removes or restores a marked wall section.
pub(super) fn interact(
    keys: Res<'_, ButtonInput<KeyCode>>,
    player: Query<'_, '_, &Transform, With<Player>>,
    mut gates: Query<'_, '_, (&mut Gate, &GlobalTransform)>,
    mut village: ResMut<'_, Village>,
    mut commands: Commands<'_, '_>,
    server: Res<'_, AssetServer>,
) {
    let Ok(player) = player.single() else { return };
    let eyes = player.translation + Vec3::Y * 1.4;
    let nearest = gates
        .iter()
        .filter(|(gate, pose)| (pose.transform_point(gate.handle).y - eyes.y).abs() < 1.4)
        .map(|(g, t)| {
            (
                g.group,
                g.label,
                g.open,
                t.transform_point(g.handle).distance(eyes),
            )
        })
        .filter(|(_, _, _, d)| *d < 3.0)
        .min_by(|a, b| a.3.total_cmp(&b.3));
    village.prompt = nearest.map_or_else(
        || "Walk to a door or window to operate it".to_owned(),
        |(_, label, open, _)| format!("E  {} {label}", if open { "Close" } else { "Open" }),
    );
    if keys.just_pressed(KeyCode::KeyE)
        && let Some((group, _, open, _)) = nearest
    {
        for (mut gate, _) in &mut gates {
            if gate.group == group {
                gate.open = !open;
            }
        }
    }
    if let Some(index) = village.sites.iter().position(|site| {
        (site.module.pose.translation.y - player.translation.y).abs() < 1.4
            && site.module.pose.translation.distance(player.translation) < 4.5
    }) {
        let present = village
            .sites
            .get(index)
            .is_some_and(|site| site.entity.is_some());
        let clear = village.sites.get(index).is_some_and(|site| {
            let pose = GlobalTransform::from(site.module.pose);
            !site
                .module
                .boxes
                .iter()
                .any(|solid| super::controller::overlaps(player.translation, *solid, &pose))
        });
        village.prompt.push_str(if present {
            "  |  X remove wall"
        } else if clear {
            "  |  X rebuild wall"
        } else {
            "  |  Step clear to rebuild wall"
        });
        if keys.just_pressed(KeyCode::KeyX)
            && (present || clear)
            && let Some(site) = village.sites.get_mut(index)
        {
            if let Some(entity) = site.entity.take() {
                commands.entity(entity).despawn();
            } else {
                site.entity = Some(spawn_module(&mut commands, &server, &site.module));
            }
        }
    }
}

/// Moves the visible leaf and its collision/acoustic children together, only while changing.
pub(super) fn animate_gates(
    time: Res<'_, Time<Real>>,
    mut gates: Query<'_, '_, (&mut Gate, &mut Transform)>,
    player: Query<'_, '_, &Transform, (With<Player>, Without<Gate>)>,
) {
    let Ok(player) = player.single() else { return };
    for (mut gate, mut pose) in &mut gates {
        let target = if gate.open { gate.open_angle } else { 0.0 };
        let next = gate.angle
            + (target - gate.angle).clamp(
                -time.delta_secs().min(0.1) * 2.5,
                time.delta_secs().min(0.1) * 2.5,
            );
        if (next - gate.angle).abs() < 0.0001 {
            continue;
        }
        let candidate = gate
            .closed
            .with_rotation(gate.closed.rotation * Quat::from_rotation_y(next));
        // Do not swing a leaf through the listener: it resumes once they move clear.
        let half = Vec3::new(0.06, gate.handle.y, gate.handle.z);
        if super::controller::overlaps(
            player.translation,
            Solid {
                center: gate.handle,
                half,
            },
            &GlobalTransform::from(candidate),
        ) {
            continue;
        }
        gate.angle = next;
        *pose = candidate;
    }
}

/// In overhead view, upper floors disappear visually but keep reflecting and muffling sound.
pub(super) fn cutaway(
    view: Res<'_, View>,
    player: Query<'_, '_, &Transform, With<Player>>,
    treatment: Res<'_, RoomTreatment>,
    mut objects: Query<
        '_,
        '_,
        (
            &Cutaway,
            &GlobalTransform,
            &mut Visibility,
            Option<&TreatmentVisual>,
        ),
    >,
) {
    let Ok(player) = player.single() else { return };
    for (level, pose, mut visibility, treatment_visual) in &mut objects {
        let in_basement = (6.0..14.0).contains(&player.translation.x)
            && (-6.0..2.0).contains(&player.translation.z)
            && pose.translation().x >= 6.0;
        let cutoff = if in_basement {
            player.translation.y
        } else {
            player.translation.y.max(0.0)
        };
        let treatment_visible = match treatment_visual {
            Some(TreatmentVisual::Furniture) => treatment.mode == 2,
            Some(TreatmentVisual::Floor(soft)) => *soft == (treatment.mode != 0),
            None => true,
        };
        let value = if treatment_visible && (view.first_person || level.0 <= cutoff + 0.4) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != value {
            *visibility = value;
        }
    }
}

/// Marks nearby editable sections, including the empty footprint after removal.
pub(super) fn markers(
    village: Res<'_, Village>,
    player: Query<'_, '_, &Transform, With<Player>>,
    mut gizmos: Gizmos<'_, '_>,
) {
    let Ok(player) = player.single() else {
        return;
    };
    for site in &village.sites {
        if (site.module.pose.translation.y - player.translation.y).abs() < 1.4
            && site.module.pose.translation.distance(player.translation) < 5.0
        {
            let pose = site
                .module
                .pose
                .with_translation(site.module.pose.translation + Vec3::Y * 1.2)
                .with_scale(Vec3::new(0.22, 2.4, 2.0));
            gizmos.cube(pose, Color::srgb(1.0, 0.76, 0.2));
        }
    }
}

#[cfg(test)]
mod treatment_tests {
    //! Live ECS treatment changes must affect geometry, not just the visible furniture.

    use super::{FurnitureFace, RoomFloor, RoomTreatment, Surface, room_treatment};
    use bevy::prelude::{App, Update, Vec3};
    use bevy_raytraced_audio_3d::RaytracedAudioSurface3d;

    /// Cycling treatment removes and restores the same furniture geometry and updates floors.
    #[test]
    fn treatment_updates_acoustic_surfaces() {
        let mut app = App::new();
        app.insert_resource(RoomTreatment { mode: 2 })
            .add_systems(Update, room_treatment);
        let points = [Vec3::ZERO, Vec3::X, Vec3::Z];
        let face = RaytracedAudioSurface3d::new(points, Surface::Fabric.material()).unwrap();
        let furniture = app.world_mut().spawn((FurnitureFace(face), face)).id();
        let floor = app.world_mut().spawn((RoomFloor(points), face)).id();
        app.update();
        assert!(
            app.world()
                .get::<RaytracedAudioSurface3d>(furniture)
                .is_some()
        );
        app.world_mut().resource_mut::<RoomTreatment>().mode = 0;
        app.update();
        assert!(
            app.world()
                .get::<RaytracedAudioSurface3d>(furniture)
                .is_none()
        );
        let hard = *app.world().get::<RaytracedAudioSurface3d>(floor).unwrap();
        app.world_mut().resource_mut::<RoomTreatment>().mode = 1;
        app.update();
        let soft = *app.world().get::<RaytracedAudioSurface3d>(floor).unwrap();
        assert!(
            soft.triangle().material().absorption().high()
                > hard.triangle().material().absorption().high()
        );
        app.world_mut().resource_mut::<RoomTreatment>().mode = 2;
        app.update();
        assert_eq!(
            app.world()
                .get::<RaytracedAudioSurface3d>(furniture)
                .unwrap()
                .triangle(),
            face.triangle()
        );
    }
}
