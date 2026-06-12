// NutriBudget entry point
// Handles CLI argument parsing, interactive prompts, and output formatting only.
// All business logic lives in lib.rs.

use clap::Parser;
use nutribudget::{
    average_daily_nutrition, build_meal_database, build_shopping_list,
    calculate_shopping_total, day_name, filter_meals, generate_week_plan, validate_inputs,
};
use std::io::Write;

// CLI argument definitions using clap derive macros.
// Every flag is optional. Any value not provided on the command line
// is collected through an interactive prompt instead.
#[derive(Parser, Debug)]
#[command(
    name = "nutribudget",
    about = "Weekly meal planner for students who know the WHO guidelines and still ate instant noodles last night.",
    long_about = None,
    version
)]
struct Args {
    // Weekly food budget in euros.
    #[arg(long, help = "Weekly budget in euros (e.g. 35)")]
    budget: Option<f64>,

    // Dietary restriction string.
    #[arg(
        long,
        help = "Dietary restriction: none | vegetarian | vegan | gluten-free | lactose-free"
    )]
    diet: Option<String>,

    // Available kitchen equipment string.
    #[arg(
        long,
        help = "Equipment: microwave-only | shared-dorm-kitchen | full-kitchen"
    )]
    equipment: Option<String>,

    // Maximum preparation time per meal in minutes.
    #[arg(long, help = "Max prep time per meal in minutes (5-60)")]
    time: Option<u32>,

    // Comma-separated list of ingredients already on hand.
    #[arg(
        long,
        help = "Ingredients you already have, comma-separated (e.g. \"pasta, eggs, onion\")"
    )]
    have: Option<String>,
}

fn main() {
    let args = Args::parse();

    // Collect any value the user did not pass as a flag through interactive prompts.
    let interactive = args.budget.is_none()
        || args.diet.is_none()
        || args.equipment.is_none()
        || args.time.is_none()
        || args.have.is_none();

    if interactive {
        println!();
        println!("NutriBudget interactive setup");
        println!("Press Enter to accept the value shown in [brackets].");
        println!();
    }

    let budget = args.budget.unwrap_or_else(prompt_budget);
    let diet = args.diet.unwrap_or_else(prompt_diet);
    let equipment = args.equipment.unwrap_or_else(prompt_equipment);
    let time = args.time.unwrap_or_else(prompt_time);
    let have = args.have.unwrap_or_else(prompt_have);

    let constraints = match validate_inputs(budget, &diet, &equipment, time, &have) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    let all_meals = build_meal_database();
    let filtered = filter_meals(&all_meals, &constraints);

    let days = match generate_week_plan(&filtered, constraints.budget_eur) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Could not generate plan: {}", e);
            std::process::exit(1);
        }
    };

    let shopping_list = build_shopping_list(&days);
    let total_cost = calculate_shopping_total(&shopping_list);
    let avg_nutrition = average_daily_nutrition(&days);

    print_header(
        constraints.equipment.display_name(),
        constraints.diet.display_name(),
        constraints.budget_eur,
    );
    print_week_schedule(&days);
    print_shopping_list(&shopping_list, total_cost);
    print_nutrition_summary(&avg_nutrition);
    print_cost_summary(total_cost, constraints.budget_eur);

    if !constraints.existing_ingredients.is_empty() {
        print_existing_ingredients_note(&constraints.existing_ingredients);
    }
}

// Prints a prompt and reads one trimmed line from standard input.
fn read_answer(prompt: &str) -> String {
    print!("{}", prompt);
    let _ = std::io::stdout().flush();
    let mut input = String::new();
    let _ = std::io::stdin().read_line(&mut input);
    input.trim().to_string()
}

// Asks for the weekly budget until a valid number in range is entered.
fn prompt_budget() -> f64 {
    loop {
        let answer = read_answer("Weekly budget in euros (10 to 200): ");
        match answer.parse::<f64>() {
            Ok(value) if (10.0..=200.0).contains(&value) => return value,
            Ok(_) => println!("  Please enter a number between 10 and 200."),
            Err(_) => println!("  That is not a number. Try something like 35."),
        }
    }
}

// Asks for a dietary restriction until a valid option is entered.
fn prompt_diet() -> String {
    loop {
        let answer = read_answer(
            "Diet (none | vegetarian | vegan | gluten-free | lactose-free) [none]: ",
        );
        if answer.is_empty() {
            return "none".to_string();
        }
        let valid = ["none", "vegetarian", "vegan", "gluten-free", "lactose-free"];
        if valid.contains(&answer.as_str()) {
            return answer;
        }
        println!("  Unknown diet. Valid options: none, vegetarian, vegan, gluten-free, lactose-free.");
    }
}

// Asks for the available equipment until a valid option is entered.
fn prompt_equipment() -> String {
    loop {
        let answer = read_answer(
            "Equipment (microwave-only | shared-dorm-kitchen | full-kitchen) [shared-dorm-kitchen]: ",
        );
        if answer.is_empty() {
            return "shared-dorm-kitchen".to_string();
        }
        let valid = ["microwave-only", "shared-dorm-kitchen", "full-kitchen"];
        if valid.contains(&answer.as_str()) {
            return answer;
        }
        println!("  Unknown equipment. Valid options: microwave-only, shared-dorm-kitchen, full-kitchen.");
    }
}

// Asks for the max prep time until a valid number in range is entered.
fn prompt_time() -> u32 {
    loop {
        let answer = read_answer("Max minutes per meal (5 to 60) [20]: ");
        if answer.is_empty() {
            return 20;
        }
        match answer.parse::<u32>() {
            Ok(value) if (5..=60).contains(&value) => return value,
            Ok(_) => println!("  Please enter a number between 5 and 60."),
            Err(_) => println!("  That is not a number. Try something like 20."),
        }
    }
}

// Asks for ingredients already on hand. An empty answer means none.
fn prompt_have() -> String {
    read_answer("Ingredients you already have, comma separated (or press Enter for none): ")
}

// Prints the top banner with the constraints used for this plan.
fn print_header(equipment: &str, diet: &str, budget: f64) {
    println!();
    println!("NutriBudget -- Week Plan");
    println!(
        "Budget: EUR {:.2} | Diet: {} | Equipment: {}",
        budget, diet, equipment
    );
    println!("{}", "-".repeat(70));
}

// Prints the 7-day meal schedule.
fn print_week_schedule(days: &[nutribudget::DayPlan]) {
    println!();
    for (i, day) in days.iter().enumerate() {
        let label = day_name(i);
        println!(
            "{}  Breakfast  {:<38} {:>3} min  EUR {:.2}",
            label,
            day.breakfast.name,
            day.breakfast.prep_time_minutes,
            day.breakfast.total_cost()
        );
        println!(
            "     Lunch      {:<38} {:>3} min  EUR {:.2}",
            day.lunch.name,
            day.lunch.prep_time_minutes,
            day.lunch.total_cost()
        );
        println!(
            "     Dinner     {:<38} {:>3} min  EUR {:.2}",
            day.dinner.name,
            day.dinner.prep_time_minutes,
            day.dinner.total_cost()
        );
        println!(
            "                Daily total                                       EUR {:.2}",
            day.daily_cost()
        );
        println!();
    }
}

// Prints the deduplicated shopping list with prices.
fn print_shopping_list(list: &[nutribudget::ShoppingItem], total: f64) {
    println!("{}", "-".repeat(70));
    println!("Shopping List (estimated total: EUR {:.2})", total);
    println!("{}", "-".repeat(70));
    for item in list {
        if item.price_eur > 0.0 {
            println!("  {:<40} {}  EUR {:.2}", item.name, item.unit, item.price_eur);
        }
    }
    println!();
}

// Prints the average daily nutrition summary.
fn print_nutrition_summary(nutrition: &nutribudget::Nutrition) {
    println!("{}", "-".repeat(70));
    println!("Daily Nutrition (7-day average)");
    println!("{}", "-".repeat(70));
    println!("  Calories  {} kcal", nutrition.calories);
    println!("  Protein   {}g", nutrition.protein_g);
    println!("  Carbs     {}g", nutrition.carbs_g);
    println!("  Fat       {}g", nutrition.fat_g);
    println!();
}

// Prints the final cost summary and a budget check message.
fn print_cost_summary(total: f64, budget: f64) {
    println!("{}", "-".repeat(70));
    if total <= budget {
        println!(
            "Estimated weekly cost: EUR {:.2} (within budget, EUR {:.2} to spare)",
            total,
            budget - total
        );
    } else {
        println!(
            "Estimated weekly cost: EUR {:.2} (EUR {:.2} over your EUR {:.2} budget)",
            total,
            total - budget,
            budget
        );
        println!("Try a shorter max time or simpler equipment to reduce costs.");
    }
    println!();
}

// Prints a note about which existing ingredients the user listed.
fn print_existing_ingredients_note(ingredients: &[String]) {
    println!("{}", "-".repeat(70));
    println!("You said you already have: {}", ingredients.join(", "));
    println!("These have been noted. The shopping list does not subtract them automatically.");
    println!("Cross off anything you already own before you shop.");
    println!();
}
