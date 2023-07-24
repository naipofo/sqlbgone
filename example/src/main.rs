fn main() {
    let preface = "
    CREATE TABLE IF NOT EXISTS package (
      u_id text NOT NULL PRIMARY KEY,
      sender text NOT NULL,
      destination_id text NOT NULL,
      size_id integer NOT NULL,
      FOREIGN KEY(destination_id) REFERENCES locker(id),
      FOREIGN KEY(size_id) REFERENCES size(id)
    );
    CREATE TABLE IF NOT EXISTS user (
      id integer PRIMARY KEY AUTOINCREMENT,
      nickname text NOT NULL
    );
    CREATE TABLE IF NOT EXISTS sender_package (
      user_id integer NOT NULL,
      package_uid text NOT NULL,
      FOREIGN KEY(user_id) REFERENCES user(id),
      FOREIGN KEY(package_uid) REFERENCES package(u_id),
      PRIMARY KEY(user_id, package_uid)
    );
    CREATE TABLE IF NOT EXISTS recipient_package (
      user_id integer NOT NULL,
      package_uid text NOT NULL,
      FOREIGN KEY(user_id) REFERENCES user(id),
      FOREIGN KEY(package_uid) REFERENCES package(u_id),
      PRIMARY KEY(user_id, package_uid)
    );
    CREATE TABLE IF NOT EXISTS event (
      u_id text NOT NULL PRIMARY KEY,
      package_uid text NOT NULL,
      event_type text NOT NULL,
      time text NOT NULL,
      -- TODO: save time as unix timestamp
      FOREIGN KEY(package_uid) REFERENCES package(u_id)
    );
    CREATE TABLE IF NOT EXISTS session (
      user_id integer NOT NULL,
      secret text NOT NULL,
      FOREIGN KEY(user_id) REFERENCES user(id)
    );
    
    CREATE TABLE IF NOT EXISTS locker (
      id text NOT NULL PRIMARY KEY,
      location text NOT NULL,
      location_human text
    );
    CREATE TABLE IF NOT EXISTS space (
      id integer PRIMARY KEY AUTOINCREMENT,
      locker_id text NOT NULL,
      size_id integer NOT NULL,
      FOREIGN KEY(locker_id) REFERENCES locker(id),
      FOREIGN KEY(size_id) REFERENCES size(id)
    );
    CREATE TABLE IF NOT EXISTS filled_space (
      space_id integer NOT NULL,
      package_uid text NOT NULL,
      unlock_code text NOT NULL,
      FOREIGN KEY(space_id) REFERENCES space(id),
      FOREIGN KEY(package_uid) REFERENCES package(u_id)
    );
    
    CREATE TABLE IF NOT EXISTS size (
      id integer PRIMARY KEY AUTOINCREMENT,
      size text NOT NULL
    );";
    let definition = sqlbgone_core::get_definition(preface).unwrap();
    println!("{:?}", definition);

    let query = "SELECT package.u_id, size_id, user_id
    FROM package
    RIGHT JOIN recipient_package
    ON package.u_id = recipient_package.package_uid
    WHERE user_id = ?";
    let query_types = sqlbgone_core::get_query(&definition, query);
    println!("{:?}", query_types);
}
