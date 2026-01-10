use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const DEFAULT_HEALTH_PATH: &str = "assets/data/health/health.json";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HealthFile {
    #[serde(default)]
    pub schedule: HashMap<String, ScheduleDay>,
    #[serde(default)]
    pub workouts: HashMap<String, serde_json::Value>, 
    #[serde(rename = "NutritionConstants", default)]
    pub nutrition_constants: HashMap<String, FoodGroup>,
    #[serde(default)]
    pub diet: DietConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScheduleDay {
    pub overview: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DietConfig {
    #[serde(default)]
    pub protein: MacroConfig,
    #[serde(default)]
    pub carbs: MacroConfig,
    #[serde(default)]
    pub fats: MacroConfig,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq)]
pub enum MacroMode {
    #[default]
    Percentage,
    Grams,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MacroConfig {
    pub percentage: f64,
    #[serde(rename = "calculated/daily")]
    pub calculated_daily: f64,
    #[serde(default)]
    pub target_grams: f64, 
    #[serde(default)]
    pub mode: MacroMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoodGroup {
    #[serde(default)]
    pub enabled: bool, 
    pub r#type: String, 
    pub unit: String,   
    #[serde(flatten)]
    pub variants: HashMap<String, FoodVariant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)] 
pub enum FoodVariant {
    Nested(HashMap<String, NutritionalInfo>),
    Direct(NutritionalInfo),
    Metadata(String), 
}

// Updated to match your specific JSON keys
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NutritionalInfo {
    // Handle both old aliases (if any exist in old files) and new keys
    #[serde(alias = "protein (g/lb)")]
    pub protein: f64,
    
    #[serde(alias = "calories (kcal/lb)")]
    pub calories: f64,

    // New Fields
    #[serde(rename = "Total Fat", default)]
    pub total_fat: f64,
    
    #[serde(rename = "Saturated Fat", default)]
    pub saturated_fat: f64,
    
    #[serde(rename = "Monounsaturated Fat", default)]
    pub monounsaturated_fat: f64,

    #[serde(rename = "Polyunsaturated Fat", default)]
    pub polyunsaturated_fat: f64,

    #[serde(rename = "Carbohydrates", default)]
    pub carbohydrates: f64, // Usually 0 for meat, but good to have

    #[serde(rename = "Cholesterol", default)]
    pub cholesterol: f64,
}