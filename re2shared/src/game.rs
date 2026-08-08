use hook86::mem::ByteSearcher;
use residat::re2::{Character, NUM_CHARACTERS, NUM_OBJECTS, OBJECT_CHARACTER_SIZE};
use thiserror::Error;

use crate::rng::RollType;

const RDT_STRING: &[u8] = b"Pl0\\Rdt\\room1000.rdt\0";

#[derive(Error, Debug)]
#[error("Unsupported game version: {0}")]
pub struct UnsupportedVersionError(String);

#[derive(Debug)]
pub struct GameVersion {
    pub version_name: &'static str,
    pub rdt_path_template: usize,
    pub characters: usize,
    pub current_char: usize,
    pub objects: usize,
    pub last_obj: usize,
    pub rng_seed: usize,
    pub igt_seconds: usize,
    pub igt_frames: usize,
    pub stage_index: usize,
    pub room_index: usize,
    pub stage_offset: usize,
    pub dummy_char: usize,
    pub keys_down: usize,
    pub keys_down_this_frame: usize,
    pub game_flags: usize,
    pub frame_tick_patch: usize,
    pub rng_roll_patch: usize,
    pub script_rng_patch: usize,
    pub script_rng_seed: usize,
    pub sound_flags: usize,
    pub game_flags2: usize,
    pub known_rng_rolls: [(usize, RollType); 127],
}

macro_rules! game_val {
    ($name:ident, $ty:ty) => {
        pub const fn $name(&self) -> $ty {
            unsafe {
                *(self.$name as *const $ty)
            }
        }
    };
}

macro_rules! game_ptr {
    ($name:ident, $ty:ty) => {
        pub const fn $name(&self) -> $ty {
            self.$name as $ty
        }
    };
}

impl GameVersion {
    pub unsafe fn detect() -> Result<&'static Self, UnsupportedVersionError> {
        // find the address of the RDT string in memory
        let [Some(rdt_path_addr)] = ByteSearcher::find_bytes_anywhere(&[RDT_STRING], None) else {
            return Err(UnsupportedVersionError(String::from("failed to find RDT string")));
        };

        log::debug!("Checking for version match");
        let rdt_path_addr = rdt_path_addr as usize;
        for version in &GAME_VERSIONS {
            log::debug!("Checking version {}", version.version_name);
            if version.rdt_path_template != rdt_path_addr {
                continue;
            }

            log::info!("Found RE2 version: {}", version.version_name);
            return Ok(version);
        }

        Err(UnsupportedVersionError(format!("RDT address {:08X}", rdt_path_addr)))
    }

    game_val!(rng_seed, u32);
    game_val!(script_rng_seed, u32);
    game_val!(keys_down, u32);
    game_val!(keys_down_this_frame, u32);
    game_val!(igt_seconds, u32);
    game_val!(igt_frames, u8);
    game_val!(stage_index, u16);
    game_val!(room_index, u16);
    game_val!(stage_offset, u32);
    game_val!(game_flags, u32);
    game_val!(sound_flags, u8);
    game_val!(game_flags2, u32);
    game_val!(last_obj, *const Character);
    game_val!(current_char, *const Character);
    game_ptr!(dummy_char, *const Character);
    game_ptr!(characters, *const *const Character);
    game_ptr!(objects, *const Character);

    pub fn is_claire(&self) -> bool {
        self.game_flags() & 0x80000000 != 0
    }

    pub fn is_b_scenario(&self) -> bool {
        self.game_flags() & 0x40000000 != 0
    }

    fn is_char_valid(&self, char: *const Character) -> bool {
        !char.is_null() && char != self.dummy_char()
    }

    pub fn is_in_game(&self) -> bool {
        unsafe {
            self.is_char_valid(*self.characters())
        }
    }

    pub fn iter_characters(&self) -> impl Iterator<Item = Option<*const Character>> {
        unsafe {
            (0..NUM_CHARACTERS).map(|i| {
                let char = *self.characters().add(i);
                self.is_char_valid(char).then_some(char)
            })
        }
    }

    pub fn iter_objects(&self) -> impl Iterator<Item = Option<*const Character>> {
        unsafe {
            (0..NUM_OBJECTS).map(|i| {
                let obj = self.objects().byte_add(OBJECT_CHARACTER_SIZE * i);
                (obj < self.last_obj()).then_some(obj)
            })
        }
    }

    pub fn known_rng_rolls(&self) -> &[(usize, RollType)] {
        &self.known_rng_rolls
    }

    pub fn current_char_index(&self) -> Option<usize> {
        if !self.is_char_valid(self.current_char()) {
            return None;
        }

        for i in 0..NUM_CHARACTERS {
            if unsafe { *self.characters().add(i) } == self.current_char() {
                return Some(i);
            }
        }

        None
    }
}

const GAME_VERSIONS: [GameVersion; 1] = [
    GameVersion {
        version_name: "sourcenext11",
        rdt_path_template: 0x0053ab98,
        characters: 0x0098a10c,
        current_char: 0x00988628,
        objects: 0x0098a61c,
        last_obj: 0x0098e51c,
        rng_seed: 0x00988610,
        igt_seconds: 0x00680588,
        igt_frames: 0x0068058c,
        stage_index: 0x0098eb14,
        room_index: 0x0098eb16,
        stage_offset: 0x0098e798,
        dummy_char: 0x0098e544,
        keys_down: 0x00988604,
        keys_down_this_frame: 0x00988608,
        game_flags: 0x00989ed0,
        //frame_tick_patch: 0x0044229b, // address we would want to patch if Rebirth didn't hook it already
        frame_tick_patch: 0x004c3c70,
        rng_roll_patch: 0x004b2a91,
        script_rng_patch: 0x004e3bec,
        script_rng_seed: 0x00695e58,
        sound_flags: 0x00989eee,
        game_flags2: 0x00989e6c,
        known_rng_rolls: [
            (0x004e3be1, RollType::Script),
            (0x00451be7, RollType::ZombieStaggerThreshold),
            (0x00451c70, RollType::ZombieStaggerThresholdHard),
            (0x00451cc0, RollType::ZombieStaggerThresholdHard),
            (0x0045592e, RollType::ZombieStaggerThreshold),
            (0x00453b02, RollType::ZombieStaggerThreshold),
            (0x00454d8b, RollType::ZombieStaggerThreshold),
            (0x00455225, RollType::ZombieStaggerThreshold),

            (0x00451c45, RollType::Partial),
            (0x00451c4f, RollType::ZombieStaggerThresholdReroll),

            (0x00451bad, RollType::Partial),
            (0x00451bb7, RollType::ZombieSpeed),

            (0x00451ad4, RollType::Partial),
            (0x00451ade, RollType::ZombieHealth),

            (0x00451b07, RollType::Partial),
            (0x00451b11, RollType::ZombieHealthAlt),

            (0x004551d5, RollType::ZombieHealth2),

            (0x00455200, RollType::ZombieSpeed2),

            (0x004522b4, RollType::Partial),
            (0x004522be, RollType::Partial),
            (0x004522ce, RollType::ZombieAppearance),

            (0x00451fae, RollType::AltZombieAppearance),
            (0x00451fe7, RollType::AltZombieAppearance2),
            (0x004552f9, RollType::ZombieAppearance2),
            (0x004526b8, RollType::ZombieLunge50),
            (0x004526e7, RollType::ZombieLunge50NotZero),
            (0x00452a76, RollType::ZombieLunge50),
            (0x00452a50, RollType::ZombieLunge25),
            (0x00453126, RollType::ZombieLunge50),
            (0x004df4eb, RollType::DestinationBlock),
            (0x00452e80, RollType::ZombieRaiseArms),
            (0x00453aea, RollType::ZombieKnockdown25),

            (0x00453ac2, RollType::Partial),
            (0x00453acc, RollType::ZombieKnockdown93),
            
            (0x004540d4, RollType::ZombieKnockdownSpeed),
            (0x00454a82, RollType::ZombieKnockdown87),
            (0x00452722, RollType::ZombieAnimationOffset),
            (0x00452c31, RollType::ZombieAnimationOffset),
            (0x004532cf, RollType::ZombieAnimationOffset),
            (0x00453d92, RollType::ZombieAnimationOffset16),
            (0x00452d71, RollType::ZombieShortMoan),
            (0x00452d7f, RollType::ZombieLongMoan),
            (0x0045329a, RollType::ZombieMoanChoice),
            (0x00452e69, RollType::ZombieArmRaiseTimer),
            (0x004546bc, RollType::ZombieEatingAnimation),
            (0x004547a7, RollType::ZombieEatingAnimation),
            (0x004547ff, RollType::ZombieTryMoan),
            (0x00454808, RollType::ZombieLongMoan50),
            (0x00454816, RollType::ZombieShortMoan50),

            (0x00454776, RollType::ZombieEatBloodSpray),
            (0x0045475c, RollType::ZombieEatBloodSpray),

            (0x00464c9a, RollType::LickerHealth),
            (0x00464c63, RollType::LickerHealth),
            (0x0046d041, RollType::LickerHealth),
            (0x0046d00a, RollType::LickerHealth),
            (0x00463abb, RollType::LickerJump37),
            (0x00466879, RollType::LickerJump37),
            (0x00463acf, RollType::LickerLick50),
            (0x0046688d, RollType::LickerLick50),
            (0x00463af3, RollType::LickerJump25),
            (0x004668b8, RollType::LickerJump25),
            (0x00466983, RollType::LickerJump25),
            (0x00466a4e, RollType::LickerJump25),
            (0x0046672c, RollType::LickerConsiderAttack),
            (0x004667ec, RollType::LickerSlash25),
            (0x0046681f, RollType::LickerSlash25),
            (0x0046db41, RollType::LickerSlash25),
            (0x0046db60, RollType::LickerSlash25),
            (0x0046db0e, RollType::LickerSlash50),
            (0x004668e3, RollType::LickerThreatened50),
            (0x0046dbb8, RollType::LickerThreatened50),
            (0x00466954, RollType::LickerJump62),
            (0x00466a23, RollType::LickerJump62),
            (0x0046dcac, RollType::LickerJump62),
            (0x0046dc09, RollType::LickerLickOrJump50),
            (0x0046dc12, RollType::LickerJump75Lick25),
            (0x0046dc33, RollType::LickerRecoil25),
            (0x0046dc48, RollType::LickerJump50LowHealth),
            (0x00465034, RollType::LickerDrool),

            (0x00484d1f, RollType::IvyHealth1),
            (0x00484cfe, RollType::IvyHealth2),
            (0x00484d3e, RollType::HealthBonus),
            (0x00484e99, RollType::IvyTentacleSet),
            (0x00484eac, RollType::IvyAmbushTentacle),

            (0x00488b4c, RollType::TentacleAnimationOffset),
            (0x00488bfb, RollType::TentacleAttachAngle),

            (0x0046f0fd, RollType::SpiderHealth1),
            (0x0046f0e9, RollType::SpiderHealth2),
            (0x0046f10f, RollType::HealthBonus),
            (0x0046f4f0, RollType::SpiderPoison3In32),
            (0x0046ff01, RollType::SpiderPoison3In32),
            (0x0047082c, RollType::SpiderPoison3In32),
            (0x0046f6f7, RollType::SpiderTurnTime),
            (0x0046f711, RollType::SpiderTurnDirection),
            (0x0046f7f6, RollType::SpiderMaxFaceTime),
            (0x0046f5f2, RollType::SpiderMaxPursueTime),
            (0x0046f903, RollType::SpiderMaxLegTurnTime),
            (0x0046f98c, RollType::SpiderMaxLegAttackTime),
            (0x0046f547, RollType::SpiderMaxIdleTime),
            (0x0046f58d, RollType::SpiderPursue50),

            (0x004d3ad1, RollType::Partial),
            (0x004d3adc, RollType::HandgunCrit),
            (0x004d3b41, RollType::Partial),
            (0x004d3b4c, RollType::HandgunCrit),

            (0x0045c196, RollType::DogHealth1),
            (0x0045e843, RollType::DogHealth1),
            (0x0045c182, RollType::DogHealth2),
            (0x0045e82f, RollType::DogHealth2),
            (0x0045c1a8, RollType::HealthBonus),
            (0x0045e855, RollType::HealthBonus),
            (0x0045c306, RollType::DogAnimationOffset1),
            (0x0045c34b, RollType::DogAnimationOffset2),
            (0x0045c35d, RollType::DogAnimationOffset3),
            (0x0045c3b8, RollType::DogAnimationOffset1),

            (0x004928e9, RollType::G2Position),
            (0x00492915, RollType::G2RepositionTime),
            (0x0049292b, RollType::G2Angle),
            (0x004920ba, RollType::G2Swipe50),
            (0x00490ad3, RollType::G2Thrust25),
            (0x004914f7, RollType::G2Slash75),

            (0x004da99e, RollType::WaterSplash),
            (0x004da9ee, RollType::WaterSplash),
            (0x004da21d, RollType::WaterSplash),
            (0x004da26d, RollType::WaterSplash),
            (0x004da558, RollType::WaterSplash),
            (0x004da5a8, RollType::WaterSplash),

            (0x004db802, RollType::SherryLegDrop),
        ],
    },
];