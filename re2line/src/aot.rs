use num_enum::{IntoPrimitive, TryFromPrimitive};

use crate::app::{DrawParams, Floor, GameObject, ObjectType, RoomId};
use crate::collision::Collider;
use crate::math::{Fixed16, Vec2};
use crate::record::State;

const TRIGGER_ON_ENTER: u8 = 0x40;


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, IntoPrimitive, TryFromPrimitive)]
#[repr(u16)]
pub enum Item {
    Empty = 0,
    Knife = 1,
    HandgunLeon = 2,
    HandgunClaire = 3,
    CustomHandgun = 4,
    Magnum = 5,
    CustomMagnum = 6,
    Shotgun = 7,
    CustomShotgun = 8,
    GrenadeLauncherExplosive = 9,
    GrenadeLauncherFlame = 10,
    GrenadeLauncherAcid = 11,
    Bowgun = 12,
    ColtSaa = 13,
    Sparkshot = 14,
    SubMachinegun = 15,
    Flamethrower = 16,
    RocketLauncher = 17,
    GatlingGun = 18,
    Beretta = 19,
    HandgunAmmo = 20,
    ShotgunShells = 21,
    MagnumRounds = 22,
    FuelTank = 23,
    ExplosiveRounds = 24,
    FlameRounds = 25,
    AcidRounds = 26,
    SmgAmmo = 27,
    SsBattery = 28,
    BowgunDarts = 29,
    InkRibbon = 30,
    SmallKey = 31,
    HandgunParts = 32,
    MagnumParts = 33,
    ShotgunParts = 34,
    FirstAidSpray = 35,
    AntiVirusBomb = 36,
    ChemicalAcW24 = 37,
    GreenHerb = 38,
    RedHerb = 39,
    BlueHerb = 40,
    GGHerb = 41,
    RGHerb = 42,
    BGHerb = 43,
    GGGHerb = 44,
    GGBHerb = 45,
    RGBHerb = 46,
    Lighter = 47,
    Lockpick = 48,
    PhotoSherry = 49,
    ValveHandle = 50,
    RedJewel = 51,
    RedKeycard = 52,
    BlueKeycard = 53,
    SerpentStone = 54,
    JaguarStone = 55,
    JaguarStoneL = 56,
    JaguarStoneR = 57,
    EagleStone = 58,
    RookPlug = 59,
    QueenPlug = 60,
    KnightPlug = 61,
    KingPlug = 62,
    WeaponBoxKey = 63,
    Detonator = 64,
    Explosive = 65,
    DetonatorAndExplosive = 66,
    SquareCrank = 67,
    FilmA = 68,
    FilmB = 69,
    FilmC = 70,
    UnicornMedal = 71,
    EagleMedal = 72,
    WolfMedal = 73,
    Cogwheel = 74,
    ManholeOpener = 75,
    MainFuse = 76,
    FuseCase = 77,
    Vaccine = 78,
    VaccineBase = 79,
    FilmD = 80,
    VaccineCart = 81,
    GVirus = 82,
    SpecialKey = 83,
    JointPlugBlue = 84,
    JointPlugRed = 85,
    Cord = 86,
    PhotoAda = 87,
    CabinKey = 88,
    SpadeKey = 89,
    DiamondKey = 90,
    HeartKey = 91,
    ClubKey = 92,
    DownKey = 93,
    UpKey = 94,
    PowerRoomKey = 95,
    MoDisk = 96,
    UmbrellaKeycard = 97,
    MasterKey = 98,
    PlatformKey = 99,
}

impl Item {
    pub const fn is_weapon(&self) -> bool {
        matches!(
            self,
            Self::Knife | Self::HandgunLeon | Self::HandgunClaire | Self::CustomHandgun
            | Self::Magnum | Self::CustomMagnum | Self::Shotgun | Self::CustomShotgun
            | Self::GrenadeLauncherExplosive | Self::GrenadeLauncherFlame | Self::GrenadeLauncherAcid
            | Self::Bowgun | Self::ColtSaa | Self::Sparkshot | Self::SubMachinegun
            | Self::Flamethrower | Self::RocketLauncher | Self::GatlingGun | Self::Beretta
        )
    }
    
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Empty => "Empty",
            Self::Knife => "Knife",
            Self::HandgunLeon => "Handgun (Leon)",
            Self::HandgunClaire => "Handgun (Claire)",
            Self::CustomHandgun => "Custom Handgun",
            Self::Magnum => "Magnum",
            Self::CustomMagnum => "Custom Magnum",
            Self::Shotgun => "Shotgun",
            Self::CustomShotgun => "Custom Shotgun",
            Self::GrenadeLauncherExplosive => "Grenade Launcher (Explosive)",
            Self::GrenadeLauncherFlame => "Grenade Launcher (Flame)",
            Self::GrenadeLauncherAcid => "Grenade Launcher (Acid)",
            Self::Bowgun => "Bowgun",
            Self::ColtSaa => "Colt SAA",
            Self::Sparkshot => "Sparkshot",
            Self::SubMachinegun => "Sub Machinegun",
            Self::Flamethrower => "Flamethrower",
            Self::RocketLauncher => "Rocket Launcher",
            Self::GatlingGun => "Gatling Gun",
            Self::Beretta => "Beretta",
            Self::HandgunAmmo => "Handgun Ammo",
            Self::ShotgunShells => "Shotgun Shells",
            Self::MagnumRounds => "Magnum Rounds",
            Self::FuelTank => "Fuel Tank",
            Self::ExplosiveRounds => "Explosive Rounds",
            Self::FlameRounds => "Flame Rounds",
            Self::AcidRounds => "Acid Rounds",
            Self::SmgAmmo => "SMG Ammo",
            Self::SsBattery => "SS Battery",
            Self::BowgunDarts => "Bowgun Darts",
            Self::InkRibbon => "Ink Ribbon",
            Self::SmallKey => "Small Key",
            Self::HandgunParts => "Handgun Parts",
            Self::MagnumParts => "Magnum Parts",
            Self::ShotgunParts => "Shotgun Parts",
            Self::FirstAidSpray => "First Aid Spray",
            Self::AntiVirusBomb => "Anti Virus Bomb",
            Self::ChemicalAcW24 => "Chemical AC-W24",
            Self::GreenHerb => "Green Herb",
            Self::RedHerb => "Red Herb",
            Self::BlueHerb => "Blue Herb",
            Self::GGHerb => "Mixed Herbs (G+G)",
            Self::RGHerb => "Mixed Herbs (R+G)",
            Self::BGHerb => "Mixed Herbs (B+G)",
            Self::GGGHerb => "Mixed Herbs (G+G+G)",
            Self::GGBHerb => "Mixed Herbs (G+G+B)",
            Self::RGBHerb => "Mixed Herbs (R+G+B)",
            Self::Lighter => "Lighter",
            Self::Lockpick => "Lockpick",
            Self::PhotoSherry => "Photo (Sherry)",
            Self::ValveHandle => "Valve Handle",
            Self::RedJewel => "Red Jewel",
            Self::RedKeycard => "Red Keycard",
            Self::BlueKeycard => "Blue Keycard",
            Self::SerpentStone => "Serpent Stone",
            Self::JaguarStone => "Jaguar Stone",
            Self::JaguarStoneL => "Jaguar Stone L",
            Self::JaguarStoneR => "Jaguar Stone R",
            Self::EagleStone => "Eagle Stone",
            Self::RookPlug => "Rook Plug",
            Self::QueenPlug => "Queen Plug",
            Self::KnightPlug => "Knight Plug",
            Self::KingPlug => "King Plug",
            Self::WeaponBoxKey => "Weapon Box Key",
            Self::Detonator => "Detonator",
            Self::Explosive => "Explosive",
            Self::DetonatorAndExplosive => "Detonator and Explosive",
            Self::SquareCrank => "Square Crank",
            Self::FilmA => "Film A",
            Self::FilmB => "Film B",
            Self::FilmC => "Film C",
            Self::FilmD => "Film D",
            Self::UnicornMedal => "Unicorn Medal",
            Self::EagleMedal => "Eagle Medal",
            Self::WolfMedal => "Wolf Medal",
            Self::Cogwheel => "Cogwheel",
            Self::ManholeOpener => "Manhole Opener",
            Self::MainFuse => "Main Fuse",
            Self::FuseCase => "Fuse Case",
            Self::Vaccine => "Vaccine",
            Self::VaccineBase => "Vaccine Base",
            Self::VaccineCart => "Vaccine Cart",
            Self::GVirus => "G-Virus",
            Self::SpecialKey => "Special Key",
            Self::JointPlugBlue => "Joint Plug Blue",
            Self::JointPlugRed => "Joint Plug Red",
            Self::Cord => "Cord",
            Self::PhotoAda => "Photo (Ada)",
            Self::CabinKey => "Cabin Key",
            Self::SpadeKey => "Spade Key",
            Self::DiamondKey => "Diamond Key",
            Self::HeartKey => "Heart Key",
            Self::ClubKey => "Club Key",
            Self::DownKey => "Down Key",
            Self::UpKey => "Up Key",
            Self::PowerRoomKey => "Power Room Key",
            Self::MoDisk => "MO Disk",
            Self::UmbrellaKeycard => "Umbrella Keycard",
            Self::MasterKey => "Master Key",
            Self::PlatformKey => "Platform Key",
        }
    }
    
    pub fn name_from_id(id: u16) -> String {
        let name = Self::try_from(id).map(|item| item.name()).unwrap_or("Unknown");
        format!("{} ({})", name, id)
    }
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum SceType {
    Auto = 0,
    Door = 1,
    Item = 2,
    Normal = 3,
    Message = 4,
    Event = 5,
    FlagChg = 6,
    Water = 7,
    Move = 8,
    Save = 9,
    ItemBox = 10,
    Damage = 11,
    Status = 12,
    Hikidashi = 13,
    Windows = 14,
    Unknown = 0xFF,
}

impl SceType {
    const fn name(&self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::Door => "Door",
            Self::Item => "Item",
            Self::Normal => "Normal",
            Self::Message => "Message",
            Self::Event => "Event",
            Self::FlagChg => "Flag Change",
            Self::Water => "Water",
            Self::Move => "Move",
            Self::Save => "Save",
            Self::ItemBox => "Item Box",
            Self::Damage => "Damage",
            Self::Status => "Status",
            Self::Hikidashi => "Hikidashi",
            Self::Windows => "Windows",
            Self::Unknown => "Unknown",
        }
    }

    const fn is_trigger(&self) -> bool {
        matches!(self, Self::Door | Self::Event | Self::FlagChg | Self::Item | Self::ItemBox | Self::Save | Self::Damage | Self::Message)
    }
}

impl From<u8> for SceType {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Auto,
            1 => Self::Door,
            2 => Self::Item,
            3 => Self::Normal,
            4 => Self::Message,
            5 => Self::Event,
            6 => Self::FlagChg,
            7 => Self::Water,
            8 => Self::Move,
            9 => Self::Save,
            10 => Self::ItemBox,
            11 => Self::Damage,
            12 => Self::Status,
            13 => Self::Hikidashi,
            14 => Self::Windows,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug)]
pub enum EntityForm {
    Door {
        next_pos_x: Fixed16,
        next_pos_y: Fixed16,
        next_pos_z: Fixed16,
        next_cdir_y: Fixed16,
        next_stage: u8,
        next_room: u8,
        next_n_floor: u8,
    },
    Item {
        i_item: u16,
        n_item: u16,
        flag: u16,
        md1: u8,
        action: u8,
    },
    Other,
}

#[derive(Debug)]
pub struct Entity {
    form: EntityForm,
    collider: Collider,
    floor: Floor,
    id: u8,
    sce: SceType,
    sat: u8,
}

impl Entity {
    pub fn new(form: EntityForm, collider: Collider, floor: u8, id: u8, sce: u8, sat: u8) -> Self {
        Self {
            form,
            collider,
            floor: Floor::Id(floor),
            id,
            sce: SceType::from(sce),
            sat,
        }
    }

    pub const fn is_trigger_on_enter(&self) -> bool {
        self.sat & TRIGGER_ON_ENTER != 0
    }

    pub fn could_trigger(&self, point: Vec2, floor: Floor) -> bool {
        self.sce.is_trigger() && self.floor.matches(floor) && self.collider.contains_point(point)
    }

    pub fn form(&self) -> &EntityForm {
        &self.form
    }

    pub fn floor(&self) -> Floor {
        self.floor
    }

    pub fn sce(&self) -> SceType {
        self.sce
    }
}

impl GameObject for Entity {
    fn object_type(&self) -> ObjectType {
        self.sce().into()
    }

    fn contains_point(&self, point: Vec2) -> bool {
        self.collider.contains_point(point)
    }

    fn name(&self) -> String {
        self.sce().name().to_string()
    }

    fn description(&self) -> String {
        let description = format!(
            "Floor: {} | ID: {} | Type: {}",
            self.floor, self.id, self.sce.name(),
        );

        match self.form {
            EntityForm::Door { next_stage, next_room, next_n_floor, .. } => {
                // FIXME: don't know the player ID here
                let room_id = RoomId::new(next_stage, next_room, 0);
                format!("{}\nTarget room: {} | Target floor: {}", description, room_id, next_n_floor)
            }
            EntityForm::Item { i_item, n_item, flag, .. } => {
                format!("{}\nItem ID: {} | Item count: {} | Flag: {}", description, i_item, n_item, flag)
            }
            EntityForm::Other => description,
        }
    }

    fn details(&self) -> Vec<(String, Vec<String>)> {
        let mut groups = self.collider.details();

        groups.push((String::from("Object"), vec![
            format!("Floor: {}", self.floor),
            format!("ID: {}", self.id),
            format!("Type: {}", self.sce.name()),
        ]));

        match self.form {
            EntityForm::Door { next_pos_x, next_pos_y, next_pos_z, next_cdir_y, next_stage, next_room, next_n_floor } => {
                groups.push((String::from("Door"), vec![
                    format!("Target X: {}", next_pos_x),
                    format!("Target Y: {}", next_pos_y),
                    format!("Target Z: {}", next_pos_z),
                    format!("Target Angle: {:.1}°", next_cdir_y.to_degrees()),
                    format!("Target Stage: {}", next_stage),
                    format!("Target Room: {}", next_room),
                    format!("Target Floor: {}", next_n_floor),
                ]));
            }
            EntityForm::Item { i_item, n_item, flag, .. } => {
                groups.push((String::from("Item"), vec![
                    format!("Type: {}", Item::name_from_id(i_item)),
                    format!("Count: {}", n_item),
                    format!("Flag: {}", flag),
                ]));
            }
            EntityForm::Other => {}
        }

        groups
    }

    fn floor(&self) -> Floor {
        self.floor
    }

    fn gui_shape(&self, draw_params: &DrawParams, state: &State) -> egui::Shape {
        let mut draw_params = draw_params.clone();
        if let Some(ref player) = state.characters()[0] {
            let trigger_point = if self.is_trigger_on_enter() {
                player.center
            } else {
                player.interaction_point()
            };
            
            if self.could_trigger(trigger_point, player.floor()) {
                draw_params.outline();
            }
        }
        
        self.collider.gui_shape(&draw_params, state)
    }
}