use binrw::binrw;
use enum_map::Enum;

#[repr(u16)]
#[binrw]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Enum)]
#[brw(repr = u16)]
pub enum RollType {
    Script = 0,
    ZombieStaggerThreshold = 1,
    ZombieHealth = 2, // double
    ZombieStaggerThresholdReroll = 3, // double; decides whether to re-roll stagger threshold with a higher base value
    ZombieAppearance = 4, // triple
    AltZombieAppearance = 5,
    ZombieLunge50 = 6,
    ZombieStaggerThresholdHard = 7,
    ZombieLunge25 = 8,
    ZombieLunge50NotZero = 9,
    DestinationBlock = 10,
    ZombieRaiseArms = 11,
    ZombieKnockdown25 = 12,
    ZombieKnockdown93 = 13, // double; 93.75
    ZombieKnockdownSpeed = 14,
    ZombieKnockdown87 = 15, // 87.5
    LickerHealth = 16,
    LickerJump37 = 17, // 37.5
    LickerJump25 = 18,
    LickerLick50 = 19,
    LickerConsiderAttack = 20,
    LickerSlash25 = 21,
    //LickerThreatened25 = 22, // this is probably not real and I made a mistake reading the code
    LickerJump62 = 23, // 62.5
    //LickerSlash75 = 24, // this is probably not real and I made a mistake reading the code
    LickerThreatened50 = 25,
    LickerLickOrJump50 = 26,
    LickerJump75Lick25 = 27,
    LickerRecoil25 = 28,
    LickerJump50LowHealth = 29,
    AltZombieAppearance2 = 30, // check is opposite of other one
    ZombieAppearance2 = 31, // only rolls once unlike the other one
    ZombieHealth2 = 32, // only rolls once unlike the other one
    LickerSlash50 = 33,
    ZombieSpeed = 34, // double
    ZombieHealthAlt = 35, // double
    IvyHealth1 = 36,
    IvyHealth2 = 37,
    IvyHealthBonus = 38,
    IvyTentacleSet = 39,
    IvyAmbushTentacle = 40,
    TentacleAnimationOffset = 41,
    TentacleAttachAngle = 42,
    Partial = 0xFFFE, // a roll that's part of a larger series of rolls and not used on its own
    Invalid = 0xFFFF,
}

impl RollType {
    pub const fn is_character_roll(&self) -> bool {
        !matches!(self, Self::Script | Self::Partial | Self::Invalid)
    }
}