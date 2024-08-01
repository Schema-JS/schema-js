use std::path::Path;

pub fn find_lowest_path(paths: &Vec<String>) -> Option<String> {
    let mut lowest_path: Option<(&str, usize)> = None;

    for path_str in paths {
        // Extract the path part from the URL
        let path = Path::new(path_str);

        // Count the components
        let component_count = path.components().count();

        // Update the lowest path if this one has fewer components
        if lowest_path.is_none() || component_count < lowest_path.unwrap().1 {
            lowest_path = Some((path_str, component_count));
        }
    }

    lowest_path.map(|(path, _)| path.to_string())
}
