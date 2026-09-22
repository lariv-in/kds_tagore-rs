use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::formula::{
    FormulaError, VariableSchema, VariableValues, eval_formula, parse_schema, schema_to_json,
    validate_formula,
};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "work_order_components")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub name: String,
    pub cost_formula: String,
    pub weight_formula: String,
    pub variables: Json,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl Model {
    pub fn variables_schema(&self) -> Result<VariableSchema, FormulaError> {
        parse_schema(&self.variables)
    }

    pub fn get_weight(&self, variables: &VariableValues) -> Result<Decimal, FormulaError> {
        let schema = self.variables_schema()?;
        eval_formula(&schema, &self.weight_formula, variables)
    }

    pub fn get_cost(&self, variables: &VariableValues) -> Result<Decimal, FormulaError> {
        let schema = self.variables_schema()?;
        eval_formula(&schema, &self.cost_formula, variables).map(|d| d.round_dp(2))
    }

    pub fn validate_formulas(&self) -> Result<(), FormulaError> {
        let schema = self.variables_schema()?;
        validate_formula(&schema, &self.weight_formula)
            .map_err(|e| FormulaError::msg(format!("weight formula: {e}")))?;
        validate_formula(&schema, &self.cost_formula)
            .map_err(|e| FormulaError::msg(format!("cost formula: {e}")))?;
        Ok(())
    }
}

pub fn schema_json_from_map(schema: &VariableSchema) -> Json {
    schema_to_json(schema)
}

pub fn empty_schema_json() -> Json {
    schema_to_json(&HashMap::new())
}

impl ActiveModelBehavior for ActiveModel {}
