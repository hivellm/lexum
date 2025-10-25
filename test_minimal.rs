use tantivy::schema::*;
use tantivy::Index;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a simple schema
    let mut schema_builder = Schema::builder();
    schema_builder.add_text_field("title", TEXT | STORED);
    schema_builder.add_text_field("content", TEXT);
    let schema = schema_builder.build();

    // Try to create an index in current directory
    let temp_dir = Path::new("./test_index");
    std::fs::create_dir_all(&temp_dir)?;
    
    println!("Creating index in: {:?}", temp_dir);
    
    match Index::create_in_dir(&temp_dir, schema) {
        Ok(_) => {
            println!("✅ Index created successfully");
            std::fs::remove_dir_all(&temp_dir)?;
        }
        Err(e) => {
            println!("❌ Failed to create index: {}", e);
            return Err(e.into());
        }
    }
    
    Ok(())
}