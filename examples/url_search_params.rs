/// `UrlSearchParams` usage example
use ars::UrlSearchParams;

fn main() {
    // Parse a query string
    let mut params = UrlSearchParams::parse("name=John&age=30&city=Tokyo");

    // Get values
    println!("name: {:?}", params.get("name")); // Some("John")
    println!("age: {:?}", params.get("age")); // Some("30")
    println!("city: {:?}", params.get("city")); // Some("Tokyo")
    println!();

    // Append a new parameter
    params.append("country", "Japan");
    println!("After append: {}", params.serialize()); // name=John&age=30&city=Tokyo&country=Japan
    println!();

    // Set (replaces all occurrences)
    params.set("age", "31");
    println!("After set: {}", params.serialize()); // name=John&age=31&city=Tokyo&country=Japan
    println!();

    // Delete a parameter
    params.delete("city");
    println!("After delete: {}", params.serialize()); // name=John&age=31&country=Japan
    println!();

    // Sort parameters alphabetically
    params.sort();
    println!("After sort: {}", params.serialize()); // age=31&country=Japan&name=John
    println!();

    // Iterate over all parameters
    println!("All parameters:");
    for (key, value) in params.iter() {
        println!("  {} = {}", key, value);
    }
}
