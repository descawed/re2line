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
    HealthBonus = 38,
    IvyTentacleSet = 39,
    IvyAmbushTentacle = 40,
    TentacleAnimationOffset = 41,
    TentacleAttachAngle = 42,
    ZombieAnimationOffset = 43,
    ZombieShortMoan = 44,
    ZombieLongMoan = 45,
    ZombieArmRaiseTimer = 46,
    ZombieMoanChoice = 47,
    ZombieEatingAnimation = 48,
    ZombieTryMoan = 49,
    ZombieLongMoan50 = 50,
    ZombieShortMoan50 = 51,
    ZombieEatBloodSpray = 52,
    LickerDrool = 53,
    SpiderHealth1 = 54,
    SpiderHealth2 = 55,
    SpiderPoison3In32 = 56,
    ZombieAnimationOffset16 = 57,
    ZombieSpeed2 = 58,
    HandgunCrit = 59,
    DogHealth1 = 60,
    DogHealth2 = 61,
    DogAnimationOffset1 = 62,
    DogAnimationOffset2 = 63,
    DogAnimationOffset3 = 64,
    SpiderTurnTime = 65,
    SpiderTurnDirection = 66,
    SpiderMaxFaceTime = 67,
    SpiderMaxPursueTime = 68,
    SpiderMaxLegTurnTime = 69,
    SpiderMaxLegAttackTime = 70,
    SpiderMaxIdleTime = 71,
    SpiderPursue50 = 72,
    G2Position = 73,
    G2Angle = 74,
    G2RepositionTime = 75,
    G2Swipe50 = 76,
    G2Slash75 = 77,
    G2Thrust25 = 78,
    Partial = 0xFFFE, // a roll that's part of a larger series of rolls and not used on its own
    Invalid = 0xFFFF,
}

impl RollType {
    pub const fn is_character_roll(&self) -> bool {
        !matches!(self, Self::Script | Self::Partial | Self::Invalid | Self::HandgunCrit)
    }
}