use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::formula::{
    FormulaError, VariableSchema, VariableValues, eval_formula, parse_schema, validate_formula,
};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "machinery_machines")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub name: String,
    pub cost_formula: String,
    pub variables: Json,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::job_machine::Entity")]
    JobMachines,
}

impl Related<super::job_machine::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::JobMachines.def()
    }
}

impl Related<super::job::Entity> for Entity {
    fn to() -> RelationDef {
        super::job_machine::Relation::Job.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::job_machine::Relation::Machine.def().rev())
    }
}

impl Model {
    pub fn variables_schema(&self) -> Result<VariableSchema, FormulaError> {
        parse_schema(&self.variables)
    }

    pub fn get_cost(&self, variables: &VariableValues) -> Result<Decimal, FormulaError> {
        let schema = self.variables_schema()?;
        eval_formula(&schema, &self.cost_formula, variables).map(|d| d.round_dp(2))
    }

    pub fn validate_formulas(&self) -> Result<(), FormulaError> {
        let schema = self.variables_schema()?;
        validate_formula(&schema, &self.cost_formula)
            .map_err(|e| FormulaError::msg(format!("cost formula: {e}")))?;
        Ok(())
    }

    pub fn formula_label(&self) -> String {
        let s = self.cost_formula.trim();
        if s.is_empty() {
            "—".into()
        } else if s.len() > 40 {
            format!("{}…", &s[..40])
        } else {
            s.to_string()
        }
    }
}

pub fn decimal_to_rupees_paisa(d: Decimal) -> (u64, u8) {
    use rust_decimal::prelude::*;
    let rupees = d.floor().to_u64().unwrap_or(0);
    let fract = d - Decimal::from(rupees);
    let paisa = (fract * Decimal::from(100)).round().to_u8().unwrap_or(0);
    (rupees, paisa.min(99))
}

pub fn rupees_paisa_to_decimal(rupees: u64, paisa: u8) -> Decimal {
    Decimal::from(rupees) + Decimal::from(paisa.min(99)) / Decimal::from(100)
}

impl ActiveModelBehavior for ActiveModel {}
