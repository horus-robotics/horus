use super::CachedPrevCapsule;
use super::answers::*;

const ANGLE_360_Q8: u32 = (360u32 << 8);
const ANGLE_360_Q16: u32 = (360u32 << 16);

fn get_start_angle_q8(nodes: &RplidarResponseCapsuleMeasurementNodes) -> u32 {
    return ((nodes.start_angle_sync_q6 & 0x7fffu16) as u32) << 2;
}

pub fn angle_diff_q8(prev_q8: u32, cur_q8: u32) -> u32 {
    if prev_q8 > cur_q8 {
        ANGLE_360_Q8 + cur_q8 - prev_q8
    } else {
        cur_q8 - prev_q8
    }
}

pub struct ParsedNode {
    pub dist_q2: u32,
    pub angle_offset_q3: u32
}

fn parse_cabin(cabin:&RplidarResponseCabinNodes) -> [ParsedNode;2] {
    let dist_q2_1 = cabin.distance_angle_1 & 0xfffc;
    let dist_q2_2 = cabin.distance_angle_2 & 0xfffc;

    let angle_offset_q3_1 =
        (cabin.offset_angles_q3 & 0xf) as u16 | ((cabin.distance_angle_1 & 0x3) << 4);
    let angle_offset_q3_2 =
        (cabin.offset_angles_q3 >> 4) as u16 | ((cabin.distance_angle_2 & 0x3) << 4);

    return [
        ParsedNode {
            dist_q2: dist_q2_1 as u32,
            angle_offset_q3: angle_offset_q3_1 as u32
        },
        ParsedNode {
            dist_q2: dist_q2_2 as u32,
            angle_offset_q3: angle_offset_q3_2 as u32
        },
    ]
}

pub fn check_sync(cur_angle_q16: u32, angle_inc_q16: u32) -> bool {
    ((cur_angle_q16 + angle_inc_q16) % ANGLE_360_Q16) < angle_inc_q16
}

fn angle_q6_to_angle_z_q14(angle_q6: u32) -> u16 {
    ((angle_q6 << 8) / 90) as u16
}

pub fn generate_quality(dist_q2: u32) -> u8 {
    if dist_q2 != 0 {
        (0x2fu8 << RPLIDAR_RESP_MEASUREMENT_QUALITY_SHIFT)
    } else {
        0u8
    }
}

pub fn generate_flag(sync: bool) -> u8 {
    if sync { 1u8 } else { 0u8 }
}

pub fn to_hq(node: &ParsedNode, cur_angle_raw_q16: u32, angle_inc_q16: u32) -> RplidarResponseMeasurementNodeHq {
    let angle_q6 = cur_angle_raw_q16 - ((node.angle_offset_q3 << 13) >> 10);
    let sync = check_sync(cur_angle_raw_q16, angle_inc_q16);

    RplidarResponseMeasurementNodeHq {
        angle_z_q14: angle_q6_to_angle_z_q14(angle_q6),
        dist_mm_q2: node.dist_q2 as u32,
        quality: generate_quality(node.dist_q2),
        flag: generate_flag(sync)
    }
}

pub fn parse_capsuled(cached_prev: &CachedPrevCapsule, nodes: RplidarResponseCapsuleMeasurementNodes) -> (Vec<RplidarResponseMeasurementNodeHq>, CachedPrevCapsule) {
    if let CachedPrevCapsule::Capsuled(prev_capsule) = cached_prev {
        let mut output_nodes : Vec<RplidarResponseMeasurementNodeHq> = Vec::with_capacity(32);

        let cur_start_angle_q8 = get_start_angle_q8(&nodes);
        let prev_start_angle_q8 = get_start_angle_q8(&prev_capsule);

        let diff_angle_q8 = angle_diff_q8(prev_start_angle_q8, cur_start_angle_q8);

        let angle_inc_q16 = diff_angle_q8 << 3;
        let mut cur_angle_raw_q16 = prev_start_angle_q8 << 8;

        for cabin in prev_capsule.cabins.iter() {
            let parsed_nodes = parse_cabin(cabin);

            for node in parsed_nodes.iter() {
                output_nodes.push(to_hq(&node, cur_angle_raw_q16, angle_inc_q16));
                cur_angle_raw_q16 += angle_inc_q16;
            }
        }

        return (output_nodes, CachedPrevCapsule::Capsuled(nodes));
    } else {
        return (Vec::new(), CachedPrevCapsule::Capsuled(nodes));
    }
}
