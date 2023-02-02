use mysql_async::{
    prelude::*,
    Conn,
    Error,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Report {
    pupil_id: usize,
    name: String,
    subject: String,
    term: String,
    content: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Reports {
    reports: Vec<Report>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Home;

#[derive(Debug)]
pub struct SubjectQuery {
    pupil_id: usize,
    name: String,
    report: String,
}

pub struct ClassQuery {
    pupil_id: usize,
    _name: String,
}

pub struct Subject {
    name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Pupil {
    id: usize,
    first_name: String,
    last_name: String,
    birthdate: String,
    class: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Pupils {
    pupils: Vec<Pupil>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Class {
    name: String,
    pupils: Option<Vec<usize>>
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Classes {
    teacher: String,
    classes: Option<Vec<Class>>,
}

pub struct DB {
    conn: mysql_async::Conn,
}

impl Classes {
    pub fn new(teacher: String) -> Classes {
        Classes { teacher, classes: None }
    }
}

impl DB {
    pub async fn new() -> Result<DB, Error> {
        let database_url =  "mysql://solly:Sollyb123@localhost:3306/rust_test";
        let pool = mysql_async::Pool::new(database_url);
        let conn = pool.get_conn().await.expect("HERE UPDATE");
        Ok(DB { conn })
    }

    pub fn conn(self) -> Conn {
        self.conn
    }
}

impl Class {
    pub fn new(name: String) -> Class {
        Class { name, pupils: None }
    }

    pub async fn pupils(&self, mut conn: Conn) -> Result<Pupils, Error> {
        let pupils = format!(r"select * from Class_{}", self.name)
            .with(())
            .map(&mut conn, |(id, first_name, last_name, birthdate, class)| Pupil {id, first_name, last_name, birthdate, class})
            .await?;
        Ok(Pupils::new(pupils))
    }

    pub async fn add_class(&self, mut conn: Conn) -> Result<(), Error> {
        self.new_class_table(&mut conn).await?;
        self.insert_classes(&mut conn).await?;

        Ok(())
    }

    // creates a table for the new class and adds Class to Classes table
    async fn new_class_table(&self, conn: &mut Conn) -> Result<(), Error> {
        let new_table = &format!(r"create table Class_{} (pupil_id int not null, name varchar(30) not null)", self.name) as &str;

        new_table.ignore(conn).await?;
        Ok(())
    }

    // adds a class to the Classes table - a list of all classes
    async fn insert_classes(&self, conn: &mut Conn) -> Result<(), Error> {
        r"insert into Classes (class_name) values (:class_name)"
            .with( params! {
                "class_name" => self.name.as_str(),
            }).ignore(conn).await?;
        Ok(())
    }

    pub async fn add_pupil(&self, pupil: Pupil, mut conn: Conn) -> Result<(), Error> {
        format!(r"insert into Class_{} (pupil_id, name) values (:pupil_id, :name)", self.name).as_str()
            .with(params!{
                "pupil_id" => pupil.id,
                "name" => format!("{} {}", pupil.first_name, pupil.last_name).as_str(),
            }).ignore(&mut conn).await?;

        Ok(())
    }

    pub async fn add_pupils(&self, pupils: Pupils, mut conn: Conn) -> Result<(), Error> {
        format!(r"insert into Class_{} (pupil_id, name) values (:pupil_id, :name)", self.name).as_str()
            .with(pupils.pupils.iter().map(|pupil| params! {
                "pupil_id" => pupil.id,
                "name" => format!("{} {}", pupil.first_name, pupil.last_name),
            })).batch(&mut conn).await?;

        Ok(())
    }

    async fn reports_query(&self, conn: &mut Conn, subject: &str, term: &str, class_query: &Vec<ClassQuery>, subject_queries: &mut Vec<(SubjectQuery, String)>) -> Result<(), Error> {

        for query in class_query {
            let mut subject_query = format!(r"select pupil_id, pupil_name, {} from {} where pupil_id = {}", term, subject, query.pupil_id)
                .with(()).map(&mut *conn, |(pupil_id, name, report)| SubjectQuery { pupil_id, name, report }).await?;
            if let Some(query) = subject_query.pop() { subject_queries.push((query, term.to_string())) }
        }
        Ok(())
    }

    pub async fn reports(&self, mut conn: Conn, subject: &str, terms: Vec<&str>) -> Result<Reports, mysql_async::Error> {
        let class_query = &format!(r"select pupil_id, name from Class_{}", self.name).as_str()
            .with(()).map(&mut conn, |(pupil_id, _name )| ClassQuery { pupil_id, _name } ).await?;

        let mut subject_queries: Vec<(SubjectQuery, String)> = vec![];
        for term in terms {
            self.reports_query(&mut conn, subject, term, class_query, &mut subject_queries).await?;
        }

        Ok(Reports::new(subject_queries.into_iter().map(|query| {
            let (term, query) = (query.1, query.0);
            Report {
                pupil_id: query.pupil_id,
                name: query.name,
                subject: subject.to_string(),
                term,
                content: query.report,
            }
        }).collect()))
    }
}

impl Pupils {
    pub fn new(pupils: Vec<Pupil>) -> Pupils {
        Pupils { pupils }
    }

    pub async fn add(&self, mut conn: Conn) -> Result<(), Error> {
        r"insert into Pupils (first_name, last_name, birthdate, class) values (:first_name, :last_name, :birthdate, :class)"
            .with(self.pupils.iter().map(|pupil| params! {
                "first_name" => pupil.first_name.as_str(),
                "last_name" => pupil.last_name.as_str(),
                "birthdate" => pupil.birthdate.as_str(),
                "class" => pupil.class.as_str(),
            })).batch(&mut conn).await?;

        Ok(())
    }
}

impl Pupil {
    pub fn new(id: usize, first_name: String, last_name: String, birthdate: String, class: String) -> Pupil {
        Pupil { id, first_name, last_name, birthdate, class }
    }

    pub async fn add(&self, mut conn: Conn) -> Result<(), mysql_async::Error> {
        r"insert into Pupils (first_name, last_name, birthdate, class) values (:first_name, :last_name, :birthdate, :class)"
            .with( params! {
                "first_name" => &self.first_name as &str,
                "last_name" => &self.last_name as &str,
                "birthdate" => &self.birthdate as &str,
                "class" => &self.class as &str,
            }).ignore(&mut conn).await?;

        Ok(())
    }
    pub async fn reports(&self, subject: &str) -> Result<Reports, Error> {
        todo!()
    }
}

impl Subject {
    pub fn new(name: String) -> Subject {
        Subject { name }
    }

    pub async fn add_subject(&self, mut conn: Conn) -> Result<(), Error> {
        self.add_to_list(&mut conn).await?;
        self.new_subject_table(&mut conn).await?;
        Ok(())
    }

    async fn add_to_list(&self, conn: &mut Conn) -> Result<(), Error> {
        r"insert into Subjects (name) values (:name)"
            .with( params! { "name" => self.name.as_str() }).ignore(conn).await?;
        Ok(())
    }

    async fn new_subject_table(&self, conn: &mut Conn) -> Result<(), Error> {
        format!(r"create table {} (pupil_id int not null, pupil_name varchar(60) not null, 
            autumn varchar(1000),
            winter varchar(1000),
            spring varchar(1000),
            summer varchar(1000),
        )", self.name).ignore(conn).await?;

        Ok(())
    }

    pub async fn add_pupils(&self, pupils: Pupils, mut conn: Conn) -> Result<(), Error> {
        format!(r"insert into {} (pupil_id, pupil_name) values (:pupil_id, :pupil_name)", self.name)
            .with(pupils.pupils.iter().map(|pupil| params! {
                "pupil_id" => pupil.id,
                "pupil_name" => format!("{} {}", pupil.first_name, pupil.last_name),
            })).batch(&mut conn).await?;

        Ok(())
    }

    pub async fn add_pupil(&self, pupil_id: usize, mut conn: Conn) -> Result<(), Error> {
        format!(r"insert into {} (pupil_id) values (:pupil_id)", self.name).as_str()
            .with(params!{
                "pupil_id" => pupil_id,
            }).ignore(&mut conn).await?;

        Ok(())
    }

}

impl Reports {
    pub fn new(reports: Vec<Report>) -> Reports {
        Reports { reports }
    }

    pub async fn update(&self, mut conn: Conn) -> Result<(), Error> {
        for report in &self.reports {
            let query = &format!("update {} set {} = '{}' where pupil_id = {}", report.subject, report.term, report.content, report.pupil_id);
            query.ignore(&mut conn).await?;
        }
        Ok(())
    }

}

impl Report {
    pub fn new(pupil_id: usize, name: String, subject: String, term: String, content: String) -> Report {
        Report {
            pupil_id,
            name,
            subject,
            term,
            content,
        }
    }

    pub async fn update(&self, mut conn: Conn) -> Result<(), mysql_async::Error> {
        format!(r"update {} set {} = :content where pupil_id = :pupil_id", self.subject, self.term).as_str()
            .with( params! {
                "content" => self.content.as_str(),
                "pupil_id" => self.pupil_id,
            }).ignore(&mut conn).await
    }
}

#[cfg(test)]
mod tests {
    use super::{ Subject, Report, Reports, Pupil, Pupils, Class };

    #[ignore]
    #[tokio::test] 
    async fn get_reports_test() {
        let database_url =  "mysql://solly:Sollyb123@localhost:3306/rust_test";
        let pool = mysql_async::Pool::new(database_url);
        let conn = pool.get_conn().await.expect("HERE UPDATE");

        let class = Class::new("0A".to_string());
        let class = class.get_reports(conn, "French", "autumn").await.expect("GET REPORTS");
        panic!("{:?}", class);
    }

    #[ignore]
    #[tokio::test]
    async fn update_report_test() {
        let database_url =  "mysql://solly:Sollyb123@localhost:3306/rust_test";
        let pool = mysql_async::Pool::new(database_url);
        let conn = pool.get_conn().await.expect("HERE UPDATE");

        let content = "Changed content".to_string();
        let (pupil_id, name, subject, term) = (2, "TestName".to_string(), "French".to_string(), "summer".to_string());

        let report = Report::new(pupil_id, name, subject, term, content);

        report.update(conn).await.expect("UPDATE");
    }

    #[ignore]
    #[tokio::test]
    async fn update_reports_test() {
        let database_url =  "mysql://solly:Sollyb123@localhost:3306/rust_test";
        let pool = mysql_async::Pool::new(database_url);
        let conn = pool.get_conn().await.expect("HERE UPDATE");

        let reports = Reports::new(vec![
            Report::new(4, "TestName".to_string(), "French".to_string(), "autumn".to_string(), "Some Test Content 1".to_string()),
            Report::new(5, "TestName".to_string(), "French".to_string(), "autumn".to_string(), "Some Test Content 2".to_string()),
            Report::new(6, "TestName".to_string(), "French".to_string(), "autumn".to_string(), "Some Test Content 3".to_string()),
            Report::new(7, "TestName".to_string(), "French".to_string(), "autumn".to_string(), "Some Test Content 4".to_string()),
        ]);

        reports.update(conn).await.expect("UPDATE REPORTS");
    }

    #[ignore]
    #[tokio::test]
    async fn add_pupils_to_class() {
        let database_url =  "mysql://solly:Sollyb123@localhost:3306/rust_test";
        let pool = mysql_async::Pool::new(database_url);
        let conn = pool.get_conn().await.expect("HERE UPDATE");

        let pupils = Pupils::new(vec![
            Pupil::new(4, "Test1".to_string(), "Test1".to_string(), "2000-01-01".to_string(), "0A".to_string()),
            Pupil::new(5, "Test2".to_string(), "Test2".to_string(), "2000-01-01".to_string(), "0A".to_string()),
            Pupil::new(6, "Test3".to_string(), "Test3".to_string(), "2000-01-01".to_string(), "0A".to_string()),
            Pupil::new(7, "Test4".to_string(), "Test4".to_string(), "2000-01-01".to_string(), "0A".to_string()),
        ]);

        let class = Class::new("0A".to_string());
        class.add_pupils(pupils, conn).await.expect("ADD PUPILS TO SUBJECT");
    }

    #[ignore]
    #[tokio::test]
    async fn new_class_table_test() {
        let database_url =  "mysql://solly:Sollyb123@localhost:3306/rust_test";
        let pool = mysql_async::Pool::new(database_url);
        let conn = pool.get_conn().await.expect("HERE UPDATE");

        let class = Class::new("0A".to_string());
        class.add_class(conn).await.expect("NEW CLASSTABLE");
    }

    #[ignore]
    #[tokio::test]
    async fn add_pupils_to_subject() {
        let database_url =  "mysql://solly:Sollyb123@localhost:3306/rust_test";
        let pool = mysql_async::Pool::new(database_url);
        let conn = pool.get_conn().await.expect("HERE UPDATE");

        let pupils = Pupils::new(vec![
            Pupil::new(4, "Test1".to_string(), "Test1".to_string(), "2000-01-01".to_string(), "0A".to_string()),
            Pupil::new(5, "Test2".to_string(), "Test2".to_string(), "2000-01-01".to_string(), "0A".to_string()),
            Pupil::new(6, "Test3".to_string(), "Test3".to_string(), "2000-01-01".to_string(), "0A".to_string()),
            Pupil::new(7, "Test4".to_string(), "Test4".to_string(), "2000-01-01".to_string(), "0A".to_string()),
        ]);

        let subject = Subject::new("French".to_string());
        subject.add_pupils(pupils, conn).await.expect("ADD PUPILS TO SUBJECT");
    }

    #[ignore]
    #[tokio::test]
    async fn add_pupils_to_school() {
        let database_url =  "mysql://solly:Sollyb123@localhost:3306/rust_test";
        let pool = mysql_async::Pool::new(database_url);
        let conn = pool.get_conn().await.expect("HERE UPDATE");

        let pupils = vec![
            Pupil::new(0, "Test1".to_string(), "Test1".to_string(), "2000-01-01".to_string(), "0A".to_string()),
            Pupil::new(0, "Test2".to_string(), "Test2".to_string(), "2000-01-01".to_string(), "0A".to_string()),
            Pupil::new(0, "Test3".to_string(), "Test3".to_string(), "2000-01-01".to_string(), "0A".to_string()),
            Pupil::new(0, "Test4".to_string(), "Test4".to_string(), "2000-01-01".to_string(), "0A".to_string()),
        ];

        let pupils = Pupils::new(pupils);
        pupils.add(conn).await.expect("ADD PUPILS");
    }

    #[ignore]
    #[tokio::test]
    async fn add_pupil_to_school() {
        let database_url =  "mysql://solly:Sollyb123@localhost:3306/rust_test";
        let pool = mysql_async::Pool::new(database_url);
        let conn = pool.get_conn().await.expect("HERE UPDATE");

        let (first_name, last_name, birthdate, class) = (
                "Test_First_Name".to_string(),
                "Test_Last_Name".to_string(),
                "2000-01-01".to_string(),
                "0A".to_string(),
            );
        let pupil = Pupil::new(0, first_name, last_name, birthdate, class);

        pupil.add(conn).await.expect("ADD PUPIL");
    }

    #[ignore]
    #[tokio::test]
    async fn add_pupil_to_subject_test() {
        let database_url =  "mysql://solly:Sollyb123@localhost:3306/rust_test";
        let pool = mysql_async::Pool::new(database_url);
        let conn = pool.get_conn().await.expect("HERE UPDATE");

       // pupil_id, subject, term, content, conn
        let (pupil_id, name) = ( 2, "French".to_string() );
        let add_pupil = Subject::new(name);

        add_pupil.add_pupil(pupil_id, conn).await.expect("ADD");
    }

}
