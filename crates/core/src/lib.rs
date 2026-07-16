//! Palimpsest core — headless-симуляция «живой карты».
//! Ничего не знает о терминале: отдаёт данные (тайлы, исходы тяг),
//! способ показа — дело клиента (`tui` и будущих).

pub mod archivist;
pub mod card;
pub mod cycle;
pub mod deck;
pub mod event;
pub mod legacy;
pub mod plane;
pub mod rune;
pub mod save;
pub mod sim;
pub mod tablet;
pub mod tile;
pub mod world;

pub use tile::TileKind;
pub use world::{DrawOutcome, World};
