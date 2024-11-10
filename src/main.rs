use std::{collections::HashMap, error::Error, fmt::format, fs::{self, File}, io, path::Path, thread::current};

use csv::{Reader, Writer};
use reqwest::{header::USER_AGENT, Client};
use serde::{Deserialize};
use scraper::{Html, Selector};


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

fn write_season_data(root_dir: &str, curr_season: &str, tournament: &[MatchResult]) -> Result<(), Box<dyn Error>> {
    let file_path = format!("{}/{}_data.csv", root_dir, curr_season.to_lowercase());
    let mut writer = Writer::from_path(file_path)?;

    writer.write_record(&["tournament_name", "game_name", "first_team_name", "score", "second_team_name", "first_team_players", "second_team_players"])?;
    for tr in tournament {
        let team1_players_str: String = tr.team1_players
            .iter()
            .map(|inner_vec| format!("[{}]", inner_vec.iter()
            .map(|s| format!("\"{}\"", s))
            .collect::<Vec<String>>().join(", ")))
            .collect::<Vec<String>>()
            .join(", ");
        
        let team1_player_formatted = format!("[{}]", team1_players_str);
        
        let team2_players_str: String = tr.team2_players
            .iter()
            .map(|inner_vec| format!("[{}]", inner_vec.iter()
            .map(|s| format!("\"{}\"", s))
            .collect::<Vec<String>>().join(", ")))
            .collect::<Vec<String>>()
            .join(", ");
        
        let team2_player_formatted = format!("[{}]", team2_players_str);
        writer.write_record(&[
            &tr.tournament_name,
            &tr.game_name,
            &tr.first_team_name,
            &tr.score,
            &tr.second_team_name,
            &team1_player_formatted,
            &team2_player_formatted,
        ])?;
    }
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
        let params = [
            ("season", season.as_str()),
            ("league[]", "WORLDS"),
            ("league[]", "IEM"),
            ("league[]", "LCK"),
            ("league[]", "LCS"),
            ("league[]", "LEC"),
            ("league[]", "LMS"),
            ("league[]", "LPL"),
            ("league[]", "MSC"),
            ("league[]", "MSI"),
            ("league[]", "AC"),            
            ("league[]", "CBLOL"),
            ("league[]", "IWC"),
            ("league[]", "LCL"),
            ("league[]", "LCO"),
            ("league[]", "LJL"),
            ("league[]", "LLA"),
            ("league[]", "LST"),
            ("league[]", "MSS"),
            ("league[]", "PCS"),
            ("league[]", "VCS"), 
        ];

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
#[derive(Debug, Deserialize)]
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
async fn get_players_from_match(score_parsed: i32, href: String, first_team_name: String, second_team_name: String) -> Result<HashMap<String, Vec<Vec<String>>>, Box<dyn Error>> {
    let client = Client::new();
    let headers = {"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36"};
    let mut result = HashMap::new();
    result.insert(first_team_name.clone(), vec![]);
    result.insert(second_team_name.clone(), vec![]);
    let mut team1_players: Vec<&str> = vec![];
    let mut team2_players: Vec<&str> = vec![];
    let a_selector = Selector::parse("a").unwrap();
    let table_selector = Selector::parse("table").unwrap();
    let tr_selector = Selector::parse("tr").unwrap();
    let td_selector = Selector::parse("td").unwrap();
    // let team
    let link = href.replace("..", "https://gol.gg").replace("page-summary", "page-game");
    let base_href_parts: Vec<&str> = link.split("/").collect();
    for num in 0..score_parsed {
        let game_number: i32 = base_href_parts.get(5).unwrap().parse::<i32>().unwrap() + num;
        let game_url = format!("https://gol.gg/game/stats/{}/page-game/", game_number);
        let response = client.get(&game_url).header(USER_AGENT, headers).send().await?;

        if !response.status().is_success() {
            panic!("Failed to query {}", &game_url);
        }
        let html_content = response.text().await?;
        let document = scraper::Html::parse_document(&html_content);
        let mut idx = 0;
        let mut table1_players = vec![];
        let mut table2_players = vec![];
        for (idx, table) in document.select(&table_selector).enumerate() {
            for (index, row_element) in table.select(&tr_selector).enumerate() {
                let columns: Vec<String> = row_element
                    .select(&td_selector)
                    .map(|col| col.text().collect::<Vec<_>>().join(""))
                    .collect();
                if index == 0 {
                    continue;
                }
                if columns.get(0).unwrap() == "Player" {
                    continue;
                }
                if idx == 0 {
                    table1_players.push(columns.get(0).unwrap().trim().to_string());
                } else {
                    table2_players.push(columns.get(0).unwrap().trim().to_string());
                }
            }
        }
        for element in document.select(&a_selector) {
            if let Some(title) = element.value().attr("title") {
                if title == format!("{} stats", first_team_name) {
                    if idx == 0 {
                        result.get_mut(&first_team_name).unwrap().push(table1_players.clone());
                    } else {
                        result.get_mut(&first_team_name).unwrap().push(table2_players.clone());
                    }
                    idx += 1;
                } else if title == format!("{} stats", second_team_name) {
                    if idx == 0 {
                        result.get_mut(&second_team_name).unwrap().push(table1_players.clone());
                    } else {
                        result.get_mut(&second_team_name).unwrap().push(table2_players.clone());
                    }
                    idx += 1;
                }
            }
        }
    }
    Ok(result)
}

fn parse_score(score: &str) -> i32 {
    let parts: Vec<&str> = score.split(" - ").collect();
    if parts.len() != 2 {
        panic!("Invalid score format: '{}'. Expected format 'X - Y'.", score);
    }

    let (x, y) = (parts[0].trim(), parts[1].trim());
    if x == "FF" || y == "FF" {
        return 0;
    }
    let x_parsed: i32 = x.parse().expect("Failed to parse first score operand");
    let y_parsed: i32 = y.parse().expect("Failed to parse second score operand");

    x_parsed + y_parsed

}

async fn get_tournament_data(tournament: String) -> Result<Vec<MatchResult>, Box<dyn Error>> {
    let client = Client::new();
    let base = "https://gol.gg/tournament/tournament-matchlist/";
    let full = format!("{}/{}/", base, tournament);

    let response = client.get(&full).send().await?;
    
    if !response.status().is_success() {
        eprintln!("Failed to fetch {}", &full);
    }
    let html_content = response.text().await?;
    let document = scraper::Html::parse_document(&html_content);

    // Create selectors for table, caption, and rows
    let table_selector = Selector::parse("table").unwrap();
    let caption_selector = Selector::parse("caption").unwrap();
    let row_selector = Selector::parse("tr").unwrap();
    let col_selector = Selector::parse("td").unwrap();
    let page_a_selector = Selector::parse("a").unwrap();

    let mut results = Vec::new();
    // Search through all tables in the page
    for table_element in document.select(&table_selector) {
        if let Some(caption) = table_element.select(&caption_selector).next() {
            let caption_text = caption.text().collect::<Vec<_>>().join("");
            if caption_text.to_lowercase().contains("results") {

                // If a table has the relevant caption, print all rows
                for (index, row_element) in table_element.select(&row_selector).enumerate() {
                    if index == 0 {
                        continue;
                    }
                    let columns: Vec<String> = row_element
                        .select(&col_selector)
                        .map(|col| col.text().collect::<Vec<_>>().join(""))
                        .collect();
                    let mut players_map = HashMap::new();
                    let score_parsed = parse_score(columns.get(2).unwrap());
                    
                    if score_parsed > 0 {
                        if let Some(a_element) = row_element.select(&page_a_selector).next() {
                            if let Some(href) = a_element.value().attr("href") {
                                players_map = get_players_from_match(score_parsed, href.to_string(), columns.get(1).unwrap().to_string(), columns.get(3).unwrap().to_string()).await.unwrap();
                            }
                        }
                    }
                    
                    if columns.len() == 7 {
                        let mut match_res = MatchResult {
                            tournament_name: tournament.to_string(),
                            game_name: columns.get(0).unwrap_or(&"N/A".to_string()).to_string(),
                            first_team_name: columns.get(1).unwrap_or(&"N/A".to_string()).to_string(),
                            score: columns.get(2).unwrap_or(&"N/A".to_string()).to_string(),
                            second_team_name: columns.get(3).unwrap_or(&"N/A".to_string()).to_string(),
                            date: columns.get(4).unwrap_or(&"N/A".to_string()).to_string(),
                            team1_players: Vec::new(),
                            team2_players: Vec::new(),
                        };
                        match_res.team1_players = players_map.get(&match_res.first_team_name).unwrap().to_vec();
                        match_res.team2_players = players_map.get(&match_res.second_team_name).unwrap().to_vec();
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
    let mut curr_season_match_results: Vec<MatchResult> = Vec::new();
    let mut curr_season = "S3".to_string();
    println!("---------- S3 ----------");
    for tournament in rdr.records() {
        let record = tournament.unwrap();
        if let (Some(first_column), Some(tournament_name)) = (record.get(0), record.get(6)) {
            // Check if the first column starts with "S4" and break if so
            if first_column != curr_season {
                if let Err(e) = write_season_data(root_dir, &curr_season, &curr_season_match_results) {
                    eprintln!("Error writing season {} tournaments data: {}", curr_season, e);
                }
                
                curr_season = record.get(0).unwrap().to_string();
                println!("---------- {} ----------", curr_season);
            }
            println!("----- {} -----", tournament_name);
            match get_tournament_data(tournament_name.to_string()).await {
                Ok(results) => {
                    curr_season_match_results.extend(results);
                }
                Err(e) => eprintln!("Error: {}", e),
            }
        }

    }
    // let _ = get_tournament_data("Battle of the Atlantic 2013").await;
}