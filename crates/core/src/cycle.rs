//! Цикл жизни листа: расцвет → увядание → умирание → смерть. Мир смертен
//! по устройству; вопрос цикла не «выживет ли», а «что успеет стать».
//! Смерть — не проигрыш, а финал истории: после неё остаются руины,
//! хроника и знание Правителя.

use serde::{Deserialize, Serialize};

// Пороги умирания и грация — не константы: их диктует Скрижаль Порога.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Bloom,
    Wane,
    Dying,
    Dead,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleState {
    /// Номер листа, с единицы.
    pub epoch: u32,
    pub phase: Phase,
    /// Тик, на котором выполнилось условие смерти (начало грации).
    pub doomed_at: Option<u64>,
}

impl CycleState {
    pub fn new(epoch: u32) -> CycleState {
        CycleState {
            epoch,
            phase: Phase::Bloom,
            doomed_at: None,
        }
    }
}

/// Итог прожитого цикла — для эпилога, легаси и будущих анлоков.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleSummary {
    pub epoch: u32,
    pub ticks_lived: u64,
    pub draws: u64,
    /// Максимум занятых клеток за жизнь — лучший час мира.
    pub peak_filled: usize,
    /// Сколько очагов поднялось за жизнь листа.
    pub hearths_founded: u32,
}
