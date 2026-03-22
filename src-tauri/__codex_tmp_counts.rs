#[path = "src/constants/categories.rs"] mod categories;
#[path = "src/constants/cars.rs"] mod cars;
#[path = "src/constants/tracks.rs"] mod tracks;
#[path = "src/constants/teams.rs"] mod teams;
#[path = "src/constants/scoring.rs"] mod scoring;
fn main() {
    println!("categories={}", categories::get_all_categories().len());
    println!("cars={}", cars::get_all_cars().len());
    println!("tracks={}", tracks::get_all_tracks().len());
    println!("teams={}", teams::count_teams());
    println!("difficulties={}", scoring::get_all_difficulties().len());
}
