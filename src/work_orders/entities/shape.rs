use std::collections::HashMap;
use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::work_orders::geometry::PrimitiveKind;
use crate::work_orders::geometry::openscad::calculate_openscad_volume;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "work_order_shapes")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub name: String,
    pub openscad_code: String,
    pub variable_names: Json,
}

impl Model {
    /// Return the variable names as Vec<String>.
    pub fn variable_names_vec(&self) -> Vec<String> {
        serde_json::from_value(self.variable_names.clone()).unwrap_or_default()
    }

    /// Check if this is a built-in standard shape primitive.
    pub fn standard_kind(&self) -> Option<PrimitiveKind> {
        PrimitiveKind::from_name(&self.name)
    }

    /// True if this is a built-in standard shape primitive.
    pub fn is_standard(&self) -> bool {
        self.standard_kind().is_some()
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::component::Entity")]
    Components,
}

impl Related<super::component::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Components.def()
    }
}

impl Model {
    /// Converts the OpenSCAD code by substituting all variables and converting to STL,
    /// which is read to calculate total volume in cubic meters (m³).
    ///
    /// Matches the exact requested signature:
    /// `get_volume(self, variables: HashMap<String, f64>) -> f64`
    pub fn get_volume(self, variables: HashMap<String, f64>) -> f64 {
        // Fast-path: standard analytical primitives require no async or thread overhead
        if let Some(kind) = self.standard_kind() {
            if let Ok(vol) = kind.calculate_volume(&variables) {
                return vol;
            }
        }

        // OpenSCAD evaluation fallback
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            match handle.runtime_flavor() {
                tokio::runtime::RuntimeFlavor::CurrentThread => {
                    std::thread::scope(|s| {
                        s.spawn(|| {
                            let rt = tokio::runtime::Builder::new_current_thread()
                                .enable_all()
                                .build()
                                .unwrap();
                            rt.block_on(self.get_volume_async(&variables)).unwrap_or(0.0)
                        })
                        .join()
                        .unwrap_or(0.0)
                    })
                }
                _ => tokio::task::block_in_place(|| {
                    handle.block_on(self.get_volume_async(&variables)).unwrap_or(0.0)
                }),
            }
        } else {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build();
            match rt {
                Ok(runtime) => runtime.block_on(self.get_volume_async(&variables)).unwrap_or(0.0),
                Err(_) => 0.0,
            }
        }
    }

    /// Asynchronous volume calculation.
    /// Uses exact analytical formula if this matches a built-in standard shape.
    /// Otherwise evaluates via OpenSCAD.
    pub async fn get_volume_async(
        &self,
        variables: &HashMap<String, f64>,
    ) -> Result<f64, crate::work_orders::geometry::openscad::OpenScadError> {
        if let Some(kind) = self.standard_kind() {
            if let Ok(vol) = kind.calculate_volume(variables) {
                return Ok(vol);
            }
        }
        calculate_openscad_volume(&self.openscad_code, variables).await
    }
}

impl ActiveModelBehavior for ActiveModel {}
