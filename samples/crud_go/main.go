package main
import (
    "database/sql"
    _ "github.com/SeleniaProject/serin-go/driver"
    "fmt"
)
func main() {
    db, _ := sql.Open("serin", "host=127.0.0.1 user=alice password=password")
    db.Exec("CREATE TABLE IF NOT EXISTS demo(id INT, name TEXT)")
    db.Exec("INSERT INTO demo VALUES(1,'hello')")
    row := db.QueryRow("SELECT name FROM demo WHERE id=1")
    var name string
    row.Scan(&name)
    fmt.Println("name:", name)
} 