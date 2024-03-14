pub mod density;
pub mod obstacle;

use self::density::DensityField;
use crate::prelude::*;

#[derive(Resource)]
pub struct CostFields {
    pub density: DensityField,
}
