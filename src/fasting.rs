use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 间歇性断食方案协议。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FastingProtocol {
    /// 16:8 经典断食法：16小时断食，8小时进食窗口（Leangains 标准模式）。
    F16_8,
    /// 18:6 进阶断食法：18小时断食，6小时进食窗口。
    F18_6,
    /// 20:4 勇士断食法：20小时断食，4小时进食窗口（Warrior Diet）。
    F20_4,
    /// 14:10 昼夜节律温和断食：14小时断食，10小时进食窗口（新手友好）。
    Circadian14_10,
    /// 自定义断食时长（小时）。
    Custom(f64),
}

impl FastingProtocol {
    /// 断食窗口目标时长（小时）。
    pub fn fasting_hours(&self) -> f64 {
        match self {
            FastingProtocol::F16_8 => 16.0,
            FastingProtocol::F18_6 => 18.0,
            FastingProtocol::F20_4 => 20.0,
            FastingProtocol::Circadian14_10 => 14.0,
            FastingProtocol::Custom(h) => *h,
        }
    }

    /// 在 24 小时周期内的进食窗口时长（小时）。
    pub fn eating_hours(&self) -> f64 {
        (24.0 - self.fasting_hours()).max(0.0)
    }

    /// 断食目标时长别名。
    pub fn target_hours(&self) -> f64 {
        self.fasting_hours()
    }

    /// 断食目标时长（分钟）。
    pub fn target_duration_minutes(&self) -> u32 {
        (self.fasting_hours() * 60.0).round() as u32
    }

    /// 方案显示名称。
    pub fn name(&self) -> &'static str {
        match self {
            FastingProtocol::F16_8 => "16:8 经典间歇性断食",
            FastingProtocol::F18_6 => "18:6 进阶断食",
            FastingProtocol::F20_4 => "20:4 勇士断食",
            FastingProtocol::Circadian14_10 => "14:10 昼夜节律断食",
            FastingProtocol::Custom(_) => "自定义断食",
        }
    }
}

/// 24 小时循环周期的阶段（断食窗口期 vs 进食窗口期）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CyclePhase {
    /// 当前处于断食窗口期中。
    Fasting { remaining_minutes: i64, progress_pct: f64 },
    /// 当前处于规定进食窗口期中。
    Eating { remaining_minutes: i64, progress_pct: f64 },
}

/// 间歇性断食会话的实时动态状态。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FastingState {
    /// 断食正在进行中，尚未达到目标时长。
    Fasting {
        elapsed_minutes: i64,
        target_minutes: i64,
        remaining_minutes: i64,
        progress_pct: f64,
    },
    /// 已超越目标断食时长，正在累积超时奖励时间。
    Overtime {
        elapsed_minutes: i64,
        target_minutes: i64,
        overtime_minutes: i64,
        progress_pct: f64,
    },
    /// 断食已由用户手动正常结束并保存归档。
    Completed {
        total_duration_minutes: i64,
        reached_target: bool,
    },
    /// 断食中途被取消或中断。
    Cancelled { duration_minutes: i64 },
}

/// 间歇性断食会话领域实体。
///
/// # 锁屏休眠与跨时区健壮性设计
/// 所有状态流转与已消耗时长均基于 `(now - started_at)` 纯函数无状态计算。
/// **完全不依赖后台轮询或 tick 定时器**。即使用户锁屏、杀死进程或电脑休眠数日，
/// 唤醒重新评估时均能立即获得数学上绝对精确的当前断食阶段。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FastingSession {
    pub id: String,
    pub user_id: String,
    pub started_at: DateTime<Utc>,
    pub target_duration_minutes: u32,
    pub completed_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
}

impl FastingSession {
    pub fn new(id: String, user_id: String, started_at: DateTime<Utc>, protocol: FastingProtocol) -> Self {
        Self {
            id,
            user_id,
            started_at,
            target_duration_minutes: protocol.target_duration_minutes(),
            completed_at: None,
            cancelled_at: None,
        }
    }

    pub fn is_active(&self) -> bool {
        self.completed_at.is_none() && self.cancelled_at.is_none()
    }

    pub fn complete(&mut self, now: DateTime<Utc>) {
        if self.is_active() {
            self.completed_at = Some(now);
        }
    }

    pub fn cancel(&mut self, now: DateTime<Utc>) {
        if self.is_active() {
            self.cancelled_at = Some(now);
        }
    }

    /// 评估相对于给定时间戳 `now` 的实时断食状态。
    /// 纯函数设计，完全独立于后台轮询。
    pub fn state_at(&self, now: DateTime<Utc>) -> FastingState {
        let target = self.target_duration_minutes as i64;

        if let Some(cancelled) = self.cancelled_at {
            let duration = (cancelled - self.started_at).num_minutes().max(0);
            return FastingState::Cancelled {
                duration_minutes: duration,
            };
        }

        if let Some(completed) = self.completed_at {
            let duration = (completed - self.started_at).num_minutes().max(0);
            return FastingState::Completed {
                total_duration_minutes: duration,
                reached_target: duration >= target,
            };
        }

        let elapsed = (now - self.started_at).num_minutes().max(0);
        let progress = if target > 0 {
            ((elapsed as f64 / target as f64) * 1000.0).round() / 10.0
        } else {
            100.0
        };

        if elapsed >= target {
            FastingState::Overtime {
                elapsed_minutes: elapsed,
                target_minutes: target,
                overtime_minutes: elapsed - target,
                progress_pct: progress,
            }
        } else {
            FastingState::Fasting {
                elapsed_minutes: elapsed,
                target_minutes: target,
                remaining_minutes: target - elapsed,
                progress_pct: progress,
            }
        }
    }

    /// 评估 24 小时循环周期阶段（断食窗口 vs 进食窗口）。
    /// 利用模运算计算：`(now - started_at) % cycle_duration`。
    pub fn cycle_phase_at(&self, now: DateTime<Utc>, protocol: FastingProtocol) -> CyclePhase {
        let elapsed_mins = (now - self.started_at).num_minutes().max(0);
        let fasting_mins = (protocol.fasting_hours() * 60.0).round() as i64;
        let eating_mins = (protocol.eating_hours() * 60.0).round() as i64;
        let cycle_mins = fasting_mins + eating_mins;

        let cycle_pos = elapsed_mins % cycle_mins;
        if cycle_pos < fasting_mins {
            let remaining = fasting_mins - cycle_pos;
            let pct = ((cycle_pos as f64 / fasting_mins as f64) * 1000.0).round() / 10.0;
            CyclePhase::Fasting {
                remaining_minutes: remaining,
                progress_pct: pct,
            }
        } else {
            let eating_pos = cycle_pos - fasting_mins;
            let remaining = eating_mins - eating_pos;
            let pct = ((eating_pos as f64 / eating_mins as f64) * 1000.0).round() / 10.0;
            CyclePhase::Eating {
                remaining_minutes: remaining,
                progress_pct: pct,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_fasting_progression_and_overtime() {
        let start = Utc::now();
        let session = FastingSession::new(
            "test_session_01".to_string(),
            "user_01".to_string(),
            start,
            FastingProtocol::F16_8, // 16 小时 = 960 分钟
        );

        // 1. 进行到 8 小时（50% 进度）
        let halfway = start + Duration::hours(8);
        let state = session.state_at(halfway);
        match state {
            FastingState::Fasting {
                elapsed_minutes,
                target_minutes,
                remaining_minutes,
                progress_pct,
            } => {
                assert_eq!(elapsed_minutes, 480);
                assert_eq!(target_minutes, 960);
                assert_eq!(remaining_minutes, 480);
                assert_eq!(progress_pct, 50.0);
            }
            _ => panic!("预期为 Fasting 状态"),
        }

        // 2. 进行到 17 小时（超时 1 小时）
        let overtime_time = start + Duration::hours(17);
        let state_overtime = session.state_at(overtime_time);
        match state_overtime {
            FastingState::Overtime {
                elapsed_minutes,
                target_minutes,
                overtime_minutes,
                ..
            } => {
                assert_eq!(elapsed_minutes, 1020);
                assert_eq!(target_minutes, 960);
                assert_eq!(overtime_minutes, 60);
            }
            _ => panic!("预期为 Overtime 状态"),
        }
    }

    #[test]
    fn test_recurring_24h_cycle_modulo() {
        let start = Utc::now();
        let session = FastingSession::new(
            "cycle_session".to_string(),
            "user_01".to_string(),
            start,
            FastingProtocol::F16_8,
        );

        // 达到 10 小时：仍在断食窗口（还剩 6 小时断食）
        let at_10h = start + Duration::hours(10);
        let phase = session.cycle_phase_at(at_10h, FastingProtocol::F16_8);
        assert_eq!(
            phase,
            CyclePhase::Fasting {
                remaining_minutes: 360,
                progress_pct: 62.5,
            }
        );

        // 达到 18 小时：进入进食窗口（18h = 16h断食 + 进食窗口已过2h -> 进食还剩 6h）
        let at_18h = start + Duration::hours(18);
        let phase_eating = session.cycle_phase_at(at_18h, FastingProtocol::F16_8);
        assert_eq!(
            phase_eating,
            CyclePhase::Eating {
                remaining_minutes: 360,
                progress_pct: 25.0,
            }
        );

        // 达到 26 小时（进入第二天周期：26 % 24 = 进入第二轮断食窗口第 2 小时）
        let at_26h = start + Duration::hours(26);
        let phase_day2 = session.cycle_phase_at(at_26h, FastingProtocol::F16_8);
        assert_eq!(
            phase_day2,
            CyclePhase::Fasting {
                remaining_minutes: 840,
                progress_pct: 12.5,
            }
        );
    }

    #[test]
    fn test_fasting_complete() {
        let start = Utc::now();
        let mut session = FastingSession::new(
            "test_session_02".to_string(),
            "user_01".to_string(),
            start,
            FastingProtocol::F16_8,
        );

        let end_time = start + Duration::hours(16);
        session.complete(end_time);

        let state = session.state_at(end_time + Duration::hours(1));
        match state {
            FastingState::Completed {
                total_duration_minutes,
                reached_target,
            } => {
                assert_eq!(total_duration_minutes, 960);
                assert!(reached_target);
            }
            _ => panic!("预期为 Completed 状态"),
        }
    }
}
