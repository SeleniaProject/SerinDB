# Serin Go Driver

This experimental Go driver wraps `pgx` to connect to SerinDB using PostgreSQL wire protocol.

```
import (
    "database/sql"
    _ "github.com/SeleniaProject/serin-go/driver"
)

func main() {
    db, _ := sql.Open("serin", "host=127.0.0.1 user=alice password=password")
    row := db.QueryRow("SELECT 1")
}
``` 