use std::{error::Error, fs::{self, File}, io, path::Path};

use csv::{Reader, Writer};
use reqwest::Client;
use serde::{Deserialize};
use scraper::{Selector};


fn create_directory(root_dir: &str) -> io::Result<()> {
    if Path::new(root_dir).exists() {
        fs::remove_dir_all(root_dir)?;
    }
    fs::create_dir(root_dir)?;
    Ok(())
}

fn write_tournaments_to_csv(root_dir: &str, tournaments: &[TournamentData]) -> Result<(), Box<dyn Error>> {
    let file_path = format!("{}/tournaments_data.csv", root_dir);
    let mut writer = Writer::from_path(file_path)?;

    writer.write_record(&["Season", "AvgTime", "FirstGame", "LastGame", "NbGames", "Region", "TournamentName"])?;

    for tr in tournaments {
        writer.write_record(&[
            &tr.season,
            &tr.avgtime,
            &tr.firstgame,
            &tr.lastgame,
            &tr.nbgames,
            &tr.region,
            &tr.trname,
        ])?;
    }
    writer.flush()?;
    println!("Wrote tournaments data to csv.");
    Ok(())
}

#[derive(Debug, Deserialize)]
struct Tournament {
    avgtime: Option<String>,
    firstgame: Option<String>,
    lastgame: Option<String>,
    nbgames: Option<String>,
    region: Option<String>,
    trname: Option<String>,
}

#[derive(Debug)]
struct TournamentData {
    season: String,
    avgtime: String,
    firstgame: String,
    lastgame: String,
    nbgames: String,
    region: String,
    trname: String,
}

async fn get_tournaments_data(root_dir: &str) ->Result<Vec<TournamentData>, Box<dyn Error>> {
    let client = Client::new();
    let url = "https://gol.gg/tournament/ajax.trlist.php";
    let mut all_tournament_data = Vec::new();

    for seasons_num in 3..15 {
        let season = format!("S{}", seasons_num);
        let params = [("season", season.as_str())];

        let response = client.post(url).form(&params).send().await?;
        if response.status().is_success() {
            let tournaments: Vec<Tournament> = response.json().await?;

            for tournament in tournaments {
                let tr_data = TournamentData {
                    season: season.clone(),
                    avgtime: tournament.avgtime.unwrap_or_else(|| "N/A".to_string()),
                    firstgame: tournament.firstgame.unwrap_or_else(|| "N/A".to_string()),
                    lastgame: tournament.lastgame.unwrap_or_else(|| "N/A".to_string()),
                    nbgames: tournament.nbgames.unwrap_or_else(|| "N/A".to_string()),
                    region: tournament.region.unwrap_or_else(|| "N/A".to_string()),
                    trname: tournament.trname.unwrap_or_else(|| "N/A".to_string()),
                };
                all_tournament_data.push(tr_data);
            }
        } else {
            eprintln!("Failed to fetch data for season {}", season)
        }
    }
    Ok(all_tournament_data)
}
#[derive(Debug)]
struct MatchResult {
    tournament_name: String,
    game_name: String,
    first_team_name: String,
    score: String,
    second_team_name: String,
    date: String,
    team1_players: Vec<Vec<String>>,
    team2_players: Vec<Vec<String>>,
}

async fn get_tournament_data(tournament: String) -> Result<Vec<MatchResult>, Box<dyn Error>> {
    let client = Client::new();
    let base = "https://gol.gg/tournament/tournament-matchlist/";
    let full = format!("{}/{}/", base, tournament);

    let response = client.get(&full).send().await?;
    
    println!("HELLO0000");
    if !response.status().is_success() {
        eprintln!("Failed to fetch {}", &full);
    }
    println!("HELLO1");
    let html_content = response.text().await?;
    let document = scraper::Html::parse_document(&html_content);

    // Create selectors for table, caption, and rows
    let table_selector = Selector::parse("table").unwrap();
    let caption_selector = Selector::parse("caption").unwrap();
    let row_selector = Selector::parse("tr").unwrap();
    let col_selector = Selector::parse("td").unwrap();

    let mut results = Vec::new();
    // Search through all tables in the page
    for table_element in document.select(&table_selector) {
        if let Some(caption) = table_element.select(&caption_selector).next() {
            let caption_text = caption.text().collect::<Vec<_>>().join("");
            if caption_text.to_lowercase().contains("results") {
                // print the caption
                println!("Found a table with caption: {}", caption_text);

                // If a table has the relevant caption, print all rows
                for row_element in table_element.select(&row_selector) {
                    let columns: Vec<String> = row_element
                        .select(&col_selector)
                        .map(|col| col.text().collect::<Vec<_>>().join(""))
                        .collect();
                    if columns.len() == 7 {
                        let match_res = MatchResult {
                            tournament_name: tournament.to_string(),
                            game_name: columns.get(0).unwrap_or(&"N/A".to_string()).to_string(),
                            first_team_name: columns.get(1).unwrap_or(&"N/A".to_string()).to_string(),
                            score: columns.get(2).unwrap_or(&"N/A".to_string()).to_string(),
                            second_team_name: columns.get(3).unwrap_or(&"N/A".to_string()).to_string(),
                            date: columns.get(4).unwrap_or(&"N/A".to_string()).to_string(),
                            team1_players: Vec::new(),
                            team2_players: Vec::new(),
                        };
                        results.push(match_res);
                    }
                }
            }
        }
    }

    Ok(results)
}

#[tokio::main]
async fn main() {
    let root_dir = "tournaments_data";

    let _ = create_directory(root_dir);

    match get_tournaments_data(root_dir).await {
        Ok(tournaments) => {
            if let Err(e) = write_tournaments_to_csv(root_dir, &tournaments) {
                eprintln!("Error writing all tournaments data to csv: {}", e);
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }

    let file = File::open(format!("{}/tournaments_data.csv", root_dir)).unwrap();
    let mut rdr = Reader::from_reader(file);
    let headers = rdr.headers().unwrap();
    let mut curr_season_match_results = Vec::new();
    for tournament in rdr.records() {
        let record = tournament.unwrap();
        if let (Some(first_column), Some(tournament_name)) = (record.get(0), record.get(6)) {
            // Check if the first column starts with "S4" and break if so
            if first_column.starts_with("S4") {
                break;
            }

            let tr_data = get_tournament_data(tournament_name.to_string()).await.unwrap();
            // Clone the tournament name and push it into the result
            curr_season_match_results.push(tr_data);
        }

    }
    println!("{:?}", curr_season_match_results);
    // let _ = get_tournament_data("Battle of the Atlantic 2013").await;
}
