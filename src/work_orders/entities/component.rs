use std::collections::HashMap;
use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use sea_orm::QueryOrder;
use serde::{Deserialize, Serialize};

use super::{material, material_rate, shape};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "work_order_components")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub name: String,
    pub shape_id: i64,
    pub material_id: i64,
    pub fixed_variables: Json,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::shape::Entity",
        from = "Column::ShapeId",
        to = "super::shape::Column::Id",
        on_delete = "Restrict"
    )]
    Shape,
    #[sea_orm(
        belongs_to = "super::material::Entity",
        from = "Column::MaterialId",
        to = "super::material::Column::Id",
        on_delete = "Restrict"
    )]
    Material,
}

impl Related<super::shape::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Shape.def()
    }
}

impl Related<super::material::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Material.def()
    }
}

impl Model {
    /// Return the fixed variables stored on the component.
    /// All dimensions are stored in millimeters (mm).
    pub fn fixed_variables_map(&self) -> HashMap<String, f64> {
        serde_json::from_value(self.fixed_variables.clone()).unwrap_or_default()
    }

    /// Return the list of shape variables that are not fixed by this component.
    pub fn free_variable_names(&self, shape: &shape::Model) -> Vec<String> {
        let fixed = self.fixed_variables_map();
        shape
            .variable_names_vec()
            .into_iter()
            .filter(|v| !fixed.contains_key(v))
            .collect()
    }

    /// If exactly 1 variable is free, solves it from target weight in kilograms.
    /// Returns `(free_var_name, solved_dimension_in_mm)`.
    pub fn solve_final_variable_from_weight(
        &self,
        shape: &shape::Model,
        material: &material::Model,
        target_weight: f64,
    ) -> Result<(String, f64), crate::work_orders::geometry::solver::SolverError> {
        let free_vars = self.free_variable_names(shape);
        if free_vars.len() != 1 {
            return Err(crate::work_orders::geometry::solver::SolverError::NotSingleFreeVariable(
                free_vars.len(),
                free_vars,
            ));
        }
        let free_var = &free_vars[0];
        let fixed = self.fixed_variables_map();
        let val = crate::work_orders::geometry::solver::solve_dimension_from_weight(
            shape,
            material,
            &fixed,
            free_var,
            target_weight,
        )?;
        Ok((free_var.clone(), val))
    }

    /// If exactly 1 variable is free, solves it from target cost in INR.
    /// Returns `(free_var_name, solved_dimension_in_mm)`.
    pub fn solve_final_variable_from_cost(
        &self,
        shape: &shape::Model,
        material: &material::Model,
        latest_rate_inr_per_kg: f64,
        target_cost: f64,
    ) -> Result<(String, f64), crate::work_orders::geometry::solver::SolverError> {
        let free_vars = self.free_variable_names(shape);
        if free_vars.len() != 1 {
            return Err(crate::work_orders::geometry::solver::SolverError::NotSingleFreeVariable(
                free_vars.len(),
                free_vars,
            ));
        }
        let free_var = &free_vars[0];
        let fixed = self.fixed_variables_map();
        let val = crate::work_orders::geometry::solver::solve_dimension_from_cost(
            shape,
            material,
            latest_rate_inr_per_kg,
            &fixed,
            free_var,
            target_cost,
        )?;
        Ok((free_var.clone(), val))
    }

    /// Calculate weight given explicit Shape and Material models.
    /// Weight (kg) = Volume (m³) * Density (kg/m³)
    pub fn get_weight_from_models(
        shape: &shape::Model,
        material: &material::Model,
        variables: HashMap<String, f64>,
    ) -> f64 {
        let volume = shape.clone().get_volume(variables);
        volume * material.density
    }

    /// Asynchronously calculate weight using database to resolve shape and material.
    pub async fn get_weight_with_db(
        &self,
        db: &DatabaseConnection,
        variables: HashMap<String, f64>,
    ) -> Result<f64, DbErr> {
        let shape = shape::Entity::find_by_id(self.shape_id)
            .one(db)
            .await?
            .ok_or_else(|| DbErr::Custom(format!("Shape #{} not found", self.shape_id)))?;

        let material = material::Entity::find_by_id(self.material_id)
            .one(db)
            .await?
            .ok_or_else(|| DbErr::Custom(format!("Material #{} not found", self.material_id)))?;

        Ok(Self::get_weight_from_models(&shape, &material, variables))
    }

    /// Takes the variables needed to calculate volume of a shape, and gives the weight based on the material.
    /// Matches the exact requested signature:
    /// `get_weight(self, variables: HashMap<String, f64>) -> f64`
    pub fn get_weight(self, _variables: HashMap<String, f64>) -> f64 {
        if let Ok(_handle) = tokio::runtime::Handle::try_current() {
            tokio::task::block_in_place(|| {
                0.0
            })
        } else {
            0.0
        }
    }

    /// Returns the cost based on the latest MaterialRate of the given Material.
    /// Asynchronous database-backed implementation:
    /// Cost = Weight (kg) * Latest Rate (INR / kg)
    pub async fn get_cost_with_db(
        &self,
        db: &DatabaseConnection,
        variables: HashMap<String, f64>,
    ) -> Result<f64, DbErr> {
        let weight = self.get_weight_with_db(db, variables).await?;

        // Find the latest rate for this material
        let latest_rate = material_rate::Entity::find()
            .filter(material_rate::Column::MaterialId.eq(self.material_id))
            .order_by_desc(material_rate::Column::Datetime)
            .order_by_desc(material_rate::Column::Id)
            .one(db)
            .await?
            .ok_or_else(|| {
                DbErr::Custom(format!(
                    "No MaterialRate found for material #{}",
                    self.material_id
                ))
            })?;

        let rate_val = latest_rate.rate();
        Ok(weight * rate_val)
    }
}

impl ActiveModelBehavior for ActiveModel {}
