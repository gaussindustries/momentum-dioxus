/**
 * 
 */
const LB_FAT_KCAL: f32 = 3500.0;

/**
 * utilzes 1lb fat kcal constant to return the kcal / day deficit
 */
pub fn calc_calorie_deficit_fat_loss(target_lbs_week: f32) -> f32{
	return (LB_FAT_KCAL * target_lbs_week) / 7.0;
}

pub fn calc_maintenence_calories(weight: f32, fat_american: bool){
	let mut weight_converted: f32 = 0.0;
	if (fat_american){
		//we're converting from imperial to metric
		weight_converted *= 0.45359237;
	} else {
		weight_converted = weight;
	}

}	
pub fn calc_calorie_surplus_muscle_gain(){

}