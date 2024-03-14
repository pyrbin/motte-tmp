use std::marker::ConstParamTy;

use crate::{
    flow_field::{
        field::{Direction, Field},
        layout::FieldLayout,
    },
    navigation::agent::{Agent, AgentRadius},
    prelude::*,
    util::math::saturate,
};

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Reflect)]
#[repr(u8)]
pub enum Cost {
    Blocked,
    #[default]
    Empty,
}

#[derive(Clone, Reflect)]
pub struct ObstacleField<const R: AgentRadius>(Field<Cost>);

pub struct ObstacleFieldLookup(HashMap<AgentRadius, Box<dyn ObstacleField>>);

impl<const R: AgentRadius> ObstacleField<R> {
    pub fn from_layout(field_layout: &FieldLayout) -> Self {
        Self(Field::new(field_layout.width(), field_layout.height(), vec![Cost::Empty; field_layout.len()]))
    }
}

#[cfg(feature = "debug")]
pub(crate) fn gizmos(mut gizmos: Gizmos, layout: Res<FieldLayout>, cost_fields: Res<super::CostFields>) {}
