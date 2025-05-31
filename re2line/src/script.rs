use binrw::binrw;

use crate::aot::{Entity, EntityForm};
use crate::collision::{CapsuleType, Collider, QuadCollider, RectCollider};
use crate::math::{Fixed16, UFixed16};

#[binrw]
#[derive(Debug)]
pub enum Instruction {
    #[brw(magic = 0x00u8)]
    Nop,
    #[brw(magic = 0x01u8)]
    EvtEnd(u8),
    #[brw(magic = 0x02u8)]
    EvtNext,
    #[brw(magic = 0x03u8)]
    EvtChain(u8),
    #[brw(magic = 0x04u8)]
    EvtExec { data: u8, go_sub: u8, scd_id: u8 },
    #[brw(magic = 0x05u8)]
    EvtKill(u8),
    #[brw(magic = 0x06u8)]
    IfElCk { align: u8, size: i16 },
    #[brw(magic = 0x07u8)]
    ElseCk { align: u8, size: i16 },
    #[brw(magic = 0x08u8)]
    EndIf(u8),
    #[brw(magic = 0x09u8)]
    Sleep,
    #[brw(magic = 0x0Au8)]
    Sleeping(i16),
    #[brw(magic = 0x0Bu8)]
    WSleep,
    #[brw(magic = 0x0Cu8)]
    WSleeping,
    #[brw(magic = 0x0Du8)]
    For { align: u8, size: u16, count: u16 },
    #[brw(magic = 0x0Eu8)]
    Next(u8),
    #[brw(magic = 0x0Fu8)]
    While { align: u8, size: u16 },
    #[brw(magic = 0x10u8)]
    EWhile(u8),
    #[brw(magic = 0x11u8)]
    Do { align: u8, size: u16 },
    #[brw(magic = 0x12u8)]
    EdWhile(u8),
    #[brw(magic = 0x13u8)]
    Switch { id: u8, size: u16 },
    #[brw(magic = 0x14u8)]
    Case { align: u8, size: u16, value: u16 },
    #[brw(magic = 0x15u8)]
    Default(u8),
    #[brw(magic = 0x16u8)]
    ESwitch(u8),
    #[brw(magic = 0x17u8)]
    Goto { ifel_ctr: u8, loop_ctr: u8, align: u8, offset: i16 },
    #[brw(magic = 0x18u8)]
    GoSub(u8),
    #[brw(magic = 0x19u8)]
    Return(u8),
    #[brw(magic = 0x1Au8)]
    Break(u8),
    #[brw(magic = 0x1Bu8)]
    For2 { align: u8, start_value: i16, align2: u8, end_value: i16 },
    #[brw(magic = 0x1Cu8)]
    BreakPoint,
    #[brw(magic = 0x1Du8)]
    WorkCopy { source: u8, destination: u8, cast: u8 },
    #[brw(magic = 0x1Eu8)]
    Nop1E,
    #[brw(magic = 0x1Fu8)]
    Nop1F,
    #[brw(magic = 0x20u8)]
    Nop20,
    #[brw(magic = 0x21u8)]
    Ck { flag: u8, id: u8, on_off: u8 },
    #[brw(magic = 0x22u8)]
    Set { flag: u8, id: u8, on_off: u8 }, // sets flag
    #[brw(magic = 0x23u8)]
    Cmp { align: u8, member: u8, operator: u8, value: i16 },
    #[brw(magic = 0x24u8)]
    Save { destination: u8, source: i16 },
    #[brw(magic = 0x25u8)]
    Copy { destination: u8, source: u8 },
    #[brw(magic = 0x26u8)]
    Calc { align: u8, operator: u8, flag: u8, value: i16 },
    #[brw(magic = 0x27u8)]
    Calc2 { operator: u8, flag: u8, value: u8 },
    #[brw(magic = 0x28u8)]
    SceRnd,
    #[brw(magic = 0x29u8)]
    CutChg(u8),
    #[brw(magic = 0x2Au8)]
    CutOld,
    #[brw(magic = 0x2Bu8)]
    MessageOn { align: u8, type_: u8, message: u8, display_time: u16 },
    #[brw(magic = 0x2Cu8)]
    AotSet {
        aot: i8,
        sce: u8,
        sat: u8,
        n_floor: u8,
        super_: u8,
        x: Fixed16,
        z: Fixed16,
        w: UFixed16,
        h: UFixed16,
        data0: u16,
        data1: u16,
        data2: u16,
    },
    #[brw(magic = 0x2Du8)]
    ObjModelSet {
        md1: u8,
        id: u8,
        ccol_old: u8,
        ccol_no: u8,
        ctex_old: u8,
        n_floor: u8,
        super_: u8,
        type_: u16,
        be_flag: u16,
        attribute: i16,
        x: Fixed16,
        y: Fixed16,
        z: Fixed16,
        dir_x: Fixed16,
        dir_y: Fixed16,
        dir_z: Fixed16,
        atari_offset_x: Fixed16,
        atari_offset_y: Fixed16,
        atari_offset_z: Fixed16,
        atari_size_x: Fixed16,
        atari_size_y: Fixed16,
        atari_size_z: Fixed16,
    },
    #[brw(magic = 0x2Eu8)]
    WorkSet { type_: u8, entity_id: u8 },
    #[brw(magic = 0x2Fu8)]
    SpeedSet { speed_id: u8, speed_value: Fixed16 },
    #[brw(magic = 0x30u8)]
    AddSpeed,
    #[brw(magic = 0x31u8)]
    AddASpeed,
    #[brw(magic = 0x32u8)]
    PosSet { align: u8, pos_x: Fixed16, pos_y: Fixed16, pos_z: Fixed16 },
    #[brw(magic = 0x33u8)]
    DirSet { align: u8, dir_x: Fixed16, dir_y: Fixed16, dir_z: Fixed16 },
    #[brw(magic = 0x34u8)]
    MemberSet { destination: u8, source: i16 },
    #[brw(magic = 0x35u8)]
    MemberSet2 { destination: u8, source: u8 },
    #[brw(magic = 0x36u8)]
    SeOn { vab: u8, edt: i16, data0: i16, x: Fixed16, y: Fixed16, z: Fixed16 },
    #[brw(magic = 0x37u8)]
    ScaIdSet { i_entry: u8, id: u16 },
    #[brw(magic = 0x38u8)]
    FlrSet { id: u8, flag: u8 },
    #[brw(magic = 0x39u8)]
    DirCk { align: u8, x: Fixed16, z: Fixed16, add: i16 },
    #[brw(magic = 0x3Au8)]
    SceEsprOn { align: u8, data0: u16, data1: u16, data2: u16, x: Fixed16, y: Fixed16, z: Fixed16, dir_y: Fixed16 },
    #[brw(magic = 0x3Bu8)]
    DoorAotSet {
        aot: u8,
        sce: u8,
        sat: u8,
        n_floor: u8,
        super_: u8,
        x: Fixed16,
        z: Fixed16,
        w: UFixed16,
        h: UFixed16,
        next_pos_x: Fixed16,
        next_pos_y: Fixed16,
        next_pos_z: Fixed16,
        next_cdir_y: Fixed16,
        next_stage: u8,
        next_room: u8,
        next_cut: u8,
        next_nfloor: u8,
        dtex_type: u8,
        door_type: u8,
        knock_type: u8,
        key_id: u8,
        key_type: u8,
        free: u8,
    },
    #[brw(magic = 0x3Cu8)]
    CutAuto(u8),
    #[brw(magic = 0x3Du8)]
    MemberCopy { destination: u8, source: u8 },
    #[brw(magic = 0x3Eu8)]
    MemberCmp { align: u8, flag: u8, operator: u8, value: i16 },
    #[brw(magic = 0x3Fu8)]
    PlcMotion { motion_id: u8, mode: u8, param: u8 },
    #[brw(magic = 0x40u8)]
    PlcDest { align: u8, animation: u8, bit: u8, x: Fixed16, z: Fixed16 },
    #[brw(magic = 0x41u8)]
    PlcNeck { op: u8, x: Fixed16, y: Fixed16, z: Fixed16, speed_x: u8, speed_z: u8 },
    #[brw(magic = 0x42u8)]
    PlcRet,
    #[brw(magic = 0x43u8)]
    PlcFlg { align: u8, data0: u8, data1: u8 },
    #[brw(magic = 0x44u8)]
    SceEmSet {
        nop: u8,
        em_no: i8,
        id: u8,
        type_: u16,
        n_floor: u8,
        sound_flg: u8,
        model_type: u8,
        em_set_flag: u8,
        pos_x: Fixed16,
        pos_y: Fixed16,
        pos_z: Fixed16,
        cdir_y: Fixed16,
        motion: i16,
        ctr_flg: i16,
    },
    #[brw(magic = 0x45u8)]
    ColChgSet { data0: u8, data1: u8, data2: u8, data3: u8 },
    #[brw(magic = 0x46u8)]
    AotReset { aot: i8, sce: u8, sat: u8, data0: i16, data1: i16, data2: i16 },
    #[brw(magic = 0x47u8)]
    AotOn(i8),
    #[brw(magic = 0x48u8)]
    SuperSet {
        align: u8,
        work: u8,
        id: u8,
        p_x: Fixed16,
        p_y: Fixed16,
        p_z: Fixed16,
        d_x: Fixed16,
        d_y: Fixed16,
        d_z: Fixed16,
    },
    #[brw(magic = 0x49u8)]
    SuperReset { align: u8, d_x: Fixed16, d_y: Fixed16, d_z: Fixed16 },
    #[brw(magic = 0x4Au8)]
    PlcGun(u8),
    #[brw(magic = 0x4Bu8)]
    CutReplace { id: u8, value: u8 },
    #[brw(magic = 0x4Cu8)]
    SceEsprKill { id: u8, tp: u8, work_kind: i8, work_no: i8 },
    #[brw(magic = 0x4Du8)]
    DoorModelSet {
        data0: u8,
        id: u8,
        ofs_y: u8,
        be_flg: u8,
        data5: u8,
        data6: u16,
        x: Fixed16,
        y: Fixed16,
        z: Fixed16,
        dir_y: Fixed16,
        data10: u16,
        data11: u16,
        data12: u16,
    },
    #[brw(magic = 0x4Eu8)]
    ItemAotSet {
        aot: u8,
        sce: u8,
        sat: u8,
        n_floor: u8,
        super_: u8,
        x: Fixed16,
        z: Fixed16,
        w: UFixed16,
        h: UFixed16,
        i_item: u16,
        n_item: u16,
        flag: u16,
        md1: u8,
        action: u8,
    },
    #[brw(magic = 0x4Fu8)]
    SceKeyCk { flag: u8, value: u16 },
    #[brw(magic = 0x50u8)]
    SceTrgCk { flag: u8, value: u16 },
    #[brw(magic = 0x51u8)]
    SceBgmControl { id: u8, op: u8, type_: u8, vol_l: u8, vol_r: u8 },
    #[brw(magic = 0x52u8)]
    SceEsprControl { id: u8, type_: u8, return_: u8, work_kind: i8, work_no: i8 },
    #[brw(magic = 0x53u8)]
    SceFadeSet { data0: u8, data1: u8, data2: u8, data3: u16 },
    #[brw(magic = 0x54u8)]
    SceEspr3dOn {
        align: u8,
        data0: u16,
        data1: u16,
        data2: u16,
        x: Fixed16,
        y: Fixed16,
        z: Fixed16,
        dir_x: Fixed16,
        dir_y: Fixed16,
        dir_z: Fixed16,
        data3: i16,
    },
    #[brw(magic = 0x55u8)]
    MemberCalc { operator: u8, flag: u16, value: i16 },
    #[brw(magic = 0x56u8)]
    MemberCalc2 { operator: u8, flag: u8, value: u8 },
    #[brw(magic = 0x57u8)]
    SceBgmTblSet { align: u8, stage: u8, room: u8, data1: u16, data2: u16 },
    #[brw(magic = 0x58u8)]
    PlcRot { id: u8, sce_free0: u16 },
    #[brw(magic = 0x59u8)]
    XaOn { mode: u8, number: u16 },
    #[brw(magic = 0x5Au8)]
    WeaponChg(u8),
    #[brw(magic = 0x5Bu8)]
    PlcCnt(u8),
    #[brw(magic = 0x5Cu8)]
    SceShakeOn { slide_ofs: i8, copy_ofs: i8 },
    #[brw(magic = 0x5Du8)]
    MizuDivSet(u8),
    #[brw(magic = 0x5Eu8)]
    KeepItemCk(u8),
    #[brw(magic = 0x5Fu8)]
    XaVol(u8),
    #[brw(magic = 0x60u8)]
    KageSet {
        work: u8,
        id: i8,
        data0: u8,
        data1: u8,
        data2: u8,
        data3: u16,
        data4: u16,
        data5: u16,
        data16: u16,
    },
    #[brw(magic = 0x61u8)]
    CutBeSet { id: u8, value: u8, on_off: u8 },
    #[brw(magic = 0x62u8)]
    SceItemLost(u8),
    #[brw(magic = 0x63u8)]
    PlcGunEff,
    #[brw(magic = 0x64u8)]
    SceEsprOn2 {
        dir_y_id2: u8,
        data1: u16,
        work_kind: u8,
        work_no: u8,
        data3: u16,
        x: Fixed16,
        y: Fixed16,
        z: Fixed16,
        dir_y: UFixed16,
    },
    #[brw(magic = 0x65u8)]
    SceEsprKill2(u8),
    #[brw(magic = 0x66u8)]
    PlcStop,
    #[brw(magic = 0x67u8)]
    AotSet4p {
        aot: u8,
        sce: u8,
        sat: u8,
        n_floor: u8,
        super_: u8,
        x0: Fixed16,
        z0: Fixed16,
        x1: Fixed16,
        z1: Fixed16,
        x2: Fixed16,
        z2: Fixed16,
        x3: Fixed16,
        z3: Fixed16,
        data0: u16,
        data1: u16,
        data2: u16,
    },
    #[brw(magic = 0x68u8)]
    DoorAotSet4p {
        aot: u8,
        sce: u8,
        sat: u8,
        n_floor: u8,
        super_: u8,
        x0: Fixed16,
        z0: Fixed16,
        x1: Fixed16,
        z1: Fixed16,
        x2: Fixed16,
        z2: Fixed16,
        x3: Fixed16,
        z3: Fixed16,
        next_pos_x: Fixed16,
        next_pos_y: Fixed16,
        next_pos_z: Fixed16,
        next_cdir_y: Fixed16,
        next_stage: u8,
        next_room: u8,
        next_cut: u8,
        next_nfloor: u8,
        dtex_type: u8,
        door_type: u8,
        knock_type: u8,
        key_id: u8,
        key_type: u8,
        free: u8,
    },
    #[brw(magic = 0x69u8)]
    ItemAotSet4p {
        aot: u8,
        sce: u8,
        sat: u8,
        n_floor: u8,
        super_: u8,
        x0: Fixed16,
        z0: Fixed16,
        x1: Fixed16,
        z1: Fixed16,
        x2: Fixed16,
        z2: Fixed16,
        x3: Fixed16,
        z3: Fixed16,
        i_item: u16,
        n_item: u16,
        flag: u16,
        md1: u8,
        action: u8,
    },
    #[brw(magic = 0x6Au8)]
    LightPosSet { align: u8, index: u8, xyz: u8, position: Fixed16 },
    #[brw(magic = 0x6Bu8)]
    LightKidoSet { index: u8, luminosity: i16 },
    #[brw(magic = 0x6Cu8)]
    RbjReset,
    #[brw(magic = 0x6Du8)]
    SceScrMove { align: u8, scrl_y: i16 },
    #[brw(magic = 0x6Eu8)]
    PartsSet { align: u8, id: i8, type_: i8, value: i16 },
    #[brw(magic = 0x6Fu8)]
    MovieOn(u8),
    #[brw(magic = 0x70u8)]
    SplcRet,
    #[brw(magic = 0x71u8)]
    SplcSce,
    #[brw(magic = 0x72u8)]
    SuperOn {
        align: u8,
        data0: u8,
        data1: u8,
        data2: i16,
        data3: i16,
        data4: i16,
        data5: i16,
        data6: i16,
        data7: i16,
    },
    #[brw(magic = 0x73u8)]
    MirrorSet {
        flag: u8,
        position: u16,
        min: u16,
        max: u16,
    },
    #[brw(magic = 0x74u8)]
    SceFadeAdjust { data0: u8, data1: i16 },
    #[brw(magic = 0x75u8)]
    SceEspr3dOn2 {
        dir_y_id2: u8,
        bit: u16,
        data_4: u16,
        data_6: u16,
        data_8: u16,
        data_a: u16,
        data_c: u16,
        data_e: u16,
        data_10: u16,
        data_12: u16,
        data_14: u16,
    },
    #[brw(magic = 0x76u8)]
    SceItemGet { id: u8, num: u8 },
    #[brw(magic = 0x77u8)]
    SceLineStart { id: u8, value: u16 },
    #[brw(magic = 0x78u8)]
    SceLineMain { id: u8, data0: i16, data1: i16 },
    #[brw(magic = 0x79u8)]
    SceLineEnd,
    #[brw(magic = 0x7Au8)]
    ScePartsBomb {
        align: u8,
        data2: u8,
        data3: u8,
        data4: u8,
        data5: u8,
        data6: i16,
        data8: i16,
        data_a: i16,
        data_c: i16,
        data_e: i16,
    },
    #[brw(magic = 0x7Bu8)]
    ScePartsDown {
        id: u8,
        x: Fixed16,
        y: Fixed16,
        z: Fixed16,
        c_dir_z: Fixed16,
        dir_x: Fixed16,
        dir_y: Fixed16,
        dir_z: Fixed16,
    },
    #[brw(magic = 0x7Cu8)]
    LightColorSet { index: u8, r: u8, g: u8, b: u8, align: u8 },
    #[brw(magic = 0x7Du8)]
    LightPosSet2 { n_cut: u8, index: u8, xyz: u8, position: i16 },
    #[brw(magic = 0x7Eu8)]
    LightKidoSet2 { align: u8, n_cut: u8, index: u8, luminosity: u16 },
    #[brw(magic = 0x7Fu8)]
    LightColorSet2 { n_cut: u8, index: u8, r: u8, g: u8, b: u8 },
    #[brw(magic = 0x80u8)]
    SeVol(u8),
    #[brw(magic = 0x81u8)]
    KeepItemCk2 { item_id: u8, quantity: u8 },
    #[brw(magic = 0x82u8)]
    SceEsprTask { work_kind: i8, work_no: i8 },
    #[brw(magic = 0x83u8)]
    PlcHeal,
    #[brw(magic = 0x84u8)]
    StMapHint(u8),
    #[brw(magic = 0x85u8)]
    SceEmPosCk { id: u8, data1: u8, att: u8, flg: u16 },
    #[brw(magic = 0x86u8)]
    PoisonCk,
    #[brw(magic = 0x87u8)]
    PoisonClr,
    #[brw(magic = 0x88u8)]
    SceItemLost2 { item_id: u8, quantity: u8 },
    #[brw(magic = 0x89u8)]
    EvtNext2,
    #[brw(magic = 0x8Au8)]
    VibSet0 { align: u8, data0: u16, data1: u16 },
    #[brw(magic = 0x8Bu8)]
    VibSet1 { id: u8, value1: u16, value2: u16 },
    #[brw(magic = 0x8Cu8)]
    VibFadeSet { align: u8, data0: u8, data1: u8, data2: u16, data3: u16 },
    #[brw(magic = 0x8Du8)]
    ItemAotSet2 {
        aot: u8,
        sce: u8,
        sat: u8,
        n_floor: u8,
        super_: u8,
        x: Fixed16,
        z: Fixed16,
        w: UFixed16,
        h: UFixed16,
        i_item: u16,
        n_item: u16,
        flag: u16,
        md1: u8,
        action: u8,
        data16: u8,
        data17: u8,
    },
    #[brw(magic = 0x8Eu8)]
    SceEmSet2 {
        align: u8,
        aot: u8,
        emd: u8,
        type_: u16,
        n_floor: u8,
        se_type: u8,
        model_type: u8,
        em_set_flag: u8,
        x: Fixed16,
        y: Fixed16,
        z: Fixed16,
        dir_y: Fixed16,
        timer0: u16,
        timer1: u16,
        data16: u16,
    },
}

impl Instruction {
    pub fn to_entity(&self) -> Option<Entity> {
        Some(match self {
            Self::AotSet { aot, sce, sat, n_floor, x, z, w, h, .. } => Entity::new(
                EntityForm::Other,
                Collider::Rect(RectCollider::new((*x).to_32(), (*z).to_32(), (*w).to_32(), (*h).to_32(), CapsuleType::None)),
                *n_floor,
                *aot as u8,
                *sce,
                *sat,
            ),
            Self::DoorAotSet { aot, sce, sat, n_floor, x, z, w, h, next_pos_x, next_pos_y, next_pos_z, next_cdir_y, next_stage, next_room, next_nfloor, .. } =>
                Entity::new(
                    EntityForm::Door {
                        next_pos_x: *next_pos_x,
                        next_pos_y: *next_pos_y,
                        next_pos_z: *next_pos_z,
                        next_cdir_y: *next_cdir_y,
                        next_stage: *next_stage,
                        next_room: *next_room,
                        next_n_floor: *next_nfloor,
                    },
                    Collider::Rect(RectCollider::new((*x).to_32(), (*z).to_32(), (*w).to_32(), (*h).to_32(), CapsuleType::None)),
                    *n_floor,
                    *aot,
                    *sce,
                    *sat,
                ),
            Self::AotSet4p { aot, sce, sat, n_floor, x0, z0, x1, z1, x2, z2, x3, z3, .. } => Entity::new(
                EntityForm::Other,
                Collider::Quad(QuadCollider::new((*x0).to_32(), (*z0).to_32(), (*x1).to_32(), (*z1).to_32(), (*x2).to_32(), (*z2).to_32(), (*x3).to_32(), (*z3).to_32())),
                *n_floor,
                *aot,
                *sce,
                *sat,
            ),
            Self::DoorAotSet4p { aot, sce, sat, n_floor, x0, z0, x1, z1, x2, z2, x3, z3, next_pos_x, next_pos_y, next_pos_z, next_cdir_y, next_stage, next_room, next_nfloor, .. } =>
                Entity::new(
                    EntityForm::Door {
                        next_pos_x: *next_pos_x,
                        next_pos_y: *next_pos_y,
                        next_pos_z: *next_pos_z,
                        next_cdir_y: *next_cdir_y,
                        next_stage: *next_stage,
                        next_room: *next_room,
                        next_n_floor: *next_nfloor,
                    },
                    Collider::Quad(QuadCollider::new((*x0).to_32(), (*z0).to_32(), (*x1).to_32(), (*z1).to_32(), (*x2).to_32(), (*z2).to_32(), (*x3).to_32(), (*z3).to_32())),
                    *n_floor,
                    *aot,
                    *sce,
                    *sat,
                ),
            Self::ItemAotSet4p { aot, sce, sat, n_floor, x0, z0, x1, z1, x2, z2, x3, z3, i_item, n_item, flag, md1, action, .. } => Entity::new(
                EntityForm::Item {
                    i_item: *i_item,
                    n_item: *n_item,
                    flag: *flag,
                    md1: *md1,
                    action: *action,
                },
                Collider::Quad(QuadCollider::new((*x0).to_32(), (*z0).to_32(), (*x1).to_32(), (*z1).to_32(), (*x2).to_32(), (*z2).to_32(), (*x3).to_32(), (*z3).to_32())),
                *n_floor,
                *aot,
                *sce,
                *sat,
            ),
            Self::ItemAotSet { aot, sce, sat, n_floor, x, z, w, h, i_item, n_item, flag, md1, action, .. } |
            Self::ItemAotSet2 { aot, sce, sat, n_floor, x, z, w, h, i_item, n_item, flag, md1, action, .. } => Entity::new(
                EntityForm::Item {
                    i_item: *i_item,
                    n_item: *n_item,
                    flag: *flag,
                    md1: *md1,
                    action: *action,
                },
                Collider::Rect(RectCollider::new((*x).to_32(), (*z).to_32(), (*w).to_32(), (*h).to_32(), CapsuleType::None)),
                *n_floor,
                *aot,
                *sce,
                *sat,
            ),
            _ => return None,
        })
    }
}