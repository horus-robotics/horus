use super::prelude::*;
use super::errors::*;
use std::f32::consts::PI;

const PI2:f32 = PI * 2f32;

fn find_first_valid_index(scan: &Vec<ScanPoint>) -> Option<usize> {
    for i in 0..scan.len() {
        if scan[i].is_valid() {
            return Some(i);
        }
    }

    return None;
}

fn find_last_valid_index(scan: &Vec<ScanPoint>) -> Option<usize> {
    for i in 0..scan.len() {
        let id = scan.len() - i - 1;

        if scan[i].is_valid() {
            return Some(id);
        }
    }

    return None;
}

fn tune_head(scan: &mut Vec<ScanPoint>, inc_origin_angle: f32) -> Result<()> {
    if let Some(head_index) = find_first_valid_index(scan) {
        let mut i = head_index;

        while i != 0 {
            i -= 1;
            let mut expect_angle = scan[i + 1].angle() - inc_origin_angle;
            if expect_angle < 0f32 {
                expect_angle = 0f32;
            }
            scan[i].set_angle(expect_angle);
        }

        return Ok(());
    } else {
        return Err(Error::OperationFail("operation failed".to_owned()));
    }
}

fn tune_tail(scan: &mut Vec<ScanPoint>, inc_origin_angle: f32) -> Result<()> {
    if let Some(tail_index) = find_last_valid_index(scan) {
        for i in tail_index+1..scan.len() {
            let mut expect_angle = scan[i - 1].angle() + inc_origin_angle;
            if expect_angle > PI2 {
                expect_angle -= PI2;
            }
            scan[i].set_angle(expect_angle);
        }

        return Ok(());
    } else {
        return Err(Error::OperationFail("operation failed".to_owned()));
    }
}

/// sort scan points
pub fn sort_scan(scan: &mut Vec<ScanPoint>) -> Result<()> {
    if scan.len() == 0 {
        return Ok(());
    }

    let inc_origin_angle = PI2 / (scan.len() as f32);
    
    tune_head(scan, inc_origin_angle)?;
    tune_tail(scan, inc_origin_angle)?;

    let front_angle = scan[0].angle();
    for i in 1..scan.len() {
        if !scan[i].is_valid() {
            let mut expect_angle = front_angle + (i as f32) * inc_origin_angle;
            if expect_angle > PI2 {
                expect_angle -= PI2;
            }
            scan[i].set_angle(expect_angle);
        }
    }

    scan.sort();
    
    return Ok(());
}
