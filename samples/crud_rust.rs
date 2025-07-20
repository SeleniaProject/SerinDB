//! Basic CRUD example using serin_rs.
use serin_rs::Client;
#[tokio::main]
async fn main() {
    let cli = Client::connect("host=127.0.0.1 user=alice password=password").await.unwrap();
    cli.execute("CREATE TABLE IF NOT EXISTS demo(id INT, name TEXT)").await.unwrap();
    cli.execute("INSERT INTO demo VALUES (1,'hello')").await.unwrap();
    let rows = cli.query("SELECT name FROM demo WHERE id=1").await.unwrap();
    println!("name:{}", rows[0].get::<usize, String>(0));
} 