use std::{error::Error, fs, io, path::Path};

use csv::Writer;
use reqwest::Client;
use serde::{Deserialize};


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
}
