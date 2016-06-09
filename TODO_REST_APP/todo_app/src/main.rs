#[macro_use] 
extern crate nickel;
extern crate rustc_serialize;
extern crate rusqlite;

use nickel::{Nickel, JsonBody, HttpRouter, MediaType,  StaticFilesHandler, NickelError};
use nickel::status::StatusCode;
use rusqlite::Connection;


#[derive(RustcDecodable, RustcEncodable, Debug)]
struct TodoItem {
    id: Option<usize>,
    text: Option<String>,
    completed: Option<bool>
}

#[derive(RustcDecodable, RustcEncodable, Debug)]
struct JsonRequest {
    todo: TodoItem
}

fn main() {


    init_db();

    let mut server = Nickel::new();
    let mut router = Nickel::router();

    router.get("/todo/", middleware! { |_, mut response|
        // List all items

        let mut msg = String::new();
        let db = Connection::open("app.db").unwrap();


        let mut statement = db.prepare("SELECT id, text, completed FROM TodoItem").unwrap();
        let mut rows = statement.query(&[]).unwrap();


        msg.push_str("{\"todo\": [");


        while let Some(result_item) = rows.next() {
            let item = result_item.unwrap();

            let id: i32 = item.get(0);
            let text: String = item.get(1);
            let completed = match item.get(2) {
                0 => "false",
                1 => "true",
                _ => panic!("Non boolean value in completed"),
            };

            let fmt_item = format!("{{\"id\": {}, \"text\": \"{}\", \"completed\": {}}},", id, text, completed);
            msg.push_str(&fmt_item);
        }

        // Remove last comma
        msg.pop();

        msg.push_str("]}");

        response.set(MediaType::Json);

        msg
    });
    
    router.get("/todo/:id", middleware! { |request, mut response|

        let id = request.param("id").unwrap();
        let db = Connection::open("app.db").unwrap();

        

        let res_item = db.query_row("SELECT id, text, completed FROM TodoItem WHERE id=$1", &[&id.to_string()], |row| {
            (row.get(0), row.get(1), row.get(2))
        });


        // Handle if the entry doesn't exist
        let item = match res_item {
            Ok(t) => t,
            Err(_) => return Err(NickelError::new(response, "Not yet creaeted", StatusCode::NotFound)),
        };
            


        let id: i32 = item.0;
        let text: String = item.1;
        let completed = match item.2 {
            0 => "false",
            1 => "true",
            _ => panic!("Non boolean value in completed")
        };


        response.set(MediaType::Json);

        format!("{{\"todo\": {{\"id\": {}, \"text\": \"{}\", \"completed\": {}}}}}", id, text, completed)
    });
    
    router.post("/todo/", middleware! { |request, mut response|
        // Insert stuff and return what we inserted

        let db = Connection::open("app.db").unwrap();

        let jsonreq = request.json_as::<JsonRequest>().unwrap();

        let todoitem = jsonreq.todo;

        let text = todoitem.text.unwrap().to_string();


        let mut insstmt = db.prepare("INSERT INTO TodoItem (text, completed)
                  VALUES ($1, $2)").unwrap();

        let id = insstmt.insert(&[&text, &0.to_string()]).unwrap();

        response.set(MediaType::Json);

        format!("{{\"todo\": {{\"id\": {}, \"text\": \"{}\", \"completed\": {}}}}}", id, text, "false")
    });

    
    router.put("/todo/:id", middleware! { |request, mut response|
        // Update the item

        let id = request.param("id").unwrap().to_owned();

        let jsonreq = request.json_as::<JsonRequest>().unwrap();

        let todoitem = jsonreq.todo;

        let newstatus = match todoitem.completed.unwrap() {
            true => 1,
            false => 0,
        };

        let db = Connection::open("app.db").unwrap();

        let res = db.execute("UPDATE TodoItem SET completed = $1 WHERE id = $2", &[&newstatus.to_string(), &id]);

        match res {
            Err(_) => return Err(NickelError::new(response, "Not yet creaeted", StatusCode::NotFound)),
            _ => (),
        };

        response.set(StatusCode::NoContent);
        ""
    });
    
    router.delete("/todo/:id", middleware! { |request, mut response|
        // Delete the item

        let id = request.param("id").unwrap();
        let db = Connection::open("app.db").unwrap();

        let res = db.execute("DELETE FROM TodoItem WHERE id = $1", &[&id]);

        match res {
            Err(_) => return Err(NickelError::new(response, "Not yet creaeted", StatusCode::NotFound)),
            _ => (),
        };

        
        response.set(StatusCode::NoContent);
        ""
    });




    server.utilize(router);
    server.utilize(StaticFilesHandler::new("assets/"));
    server.listen("127.0.0.1:6767");
}


fn init_db() {
    let db = Connection::open("app.db").unwrap();

    db.execute("CREATE TABLE IF NOT EXISTS TodoItem (
                  id              INTEGER PRIMARY KEY,
                  text            TEXT,
                  completed       INTEGER NOT NULL
                  )", &[]).unwrap();
}
