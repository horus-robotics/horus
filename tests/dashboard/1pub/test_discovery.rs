use std::path::Path;

fn main() {
    println!("Checking /dev/shm/horus/sessions...");
    
    let sessions_dir = Path::new("/dev/shm/horus/sessions");
    if !sessions_dir.exists() {
        println!("Sessions directory does not exist!");
        return;
    }
    
    match std::fs::read_dir(sessions_dir) {
        Ok(entries) => {
            for entry in entries {
                if let Ok(entry) = entry {
                    let session_path = entry.path();
                    println!("Found session: {:?}", session_path);
                    
                    let topics_path = session_path.join("topics");
                    if topics_path.exists() {
                        println!("  Topics directory exists: {:?}", topics_path);
                        
                        match std::fs::read_dir(&topics_path) {
                            Ok(topic_entries) => {
                                for topic_entry in topic_entries {
                                    if let Ok(topic_entry) = topic_entry {
                                        println!("    Topic file: {:?}", topic_entry.file_name());
                                    }
                                }
                            }
                            Err(e) => println!("  Error reading topics: {}", e),
                        }
                    } else {
                        println!("  No topics directory");
                    }
                }
            }
        }
        Err(e) => println!("Error reading sessions directory: {}", e),
    }
}
