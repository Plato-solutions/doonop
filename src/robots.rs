use cylon::{Compiler, Cylon};
use std::{collections::HashMap, io};
use url::Url;

#[derive(Default, Debug)]
pub struct RobotsMap {
    map: HashMap<(Domain, Robot), RobotsVerifier>,
    files: HashMap<Robot, Vec<u8>>,
}

type Domain = String;
type Robot = String;

impl RobotsMap {
    pub async fn is_allowed(&mut self, robot: &str, url: Url) -> io::Result<bool> {
        let domain = match url.domain() {
            Some(domain) => domain,
            None => return Ok(true),
        };

        let key = (domain.to_string(), robot.to_string());

        if let Some(verifier) = self.map.get(&key) {
            return Ok(verifier.is_allowed(&url));
        }

        if let Some(file) = self.files.get(robot) {
            let verifier = RobotsVerifier::new(robot, file).await;
            self.map.insert(key.clone(), verifier);

            let verifier = self.map.get(&key).unwrap();
            return Ok(verifier.is_allowed(&url));
        }

        let mut robot_url = url.clone();
        robot_url.set_fragment(None);
        robot_url.set_path("");
        robot_url.set_query(None);
        let robot_url = robot_url.join("/robots.txt").map_err(|_| {
            io::Error::new(
                io::ErrorKind::Other,
                "inner-error: Failed to build a error of url",
            )
        })?;

        let res = reqwest::get(robot_url)
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
            .bytes()
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        let verifier = RobotsVerifier::new(robot, res).await;
        self.map.insert(key.clone(), verifier);

        let verifier = self.map.get(&key).unwrap();
        return Ok(verifier.is_allowed(&url));
    }
}

#[derive(Debug)]
pub struct RobotsVerifier {
    robot: String,
    compiled_data: Cylon,
}

impl RobotsVerifier {
    pub async fn new(robot: impl Into<String>, file: impl AsRef<[u8]>) -> Self {
        let robot = robot.into();
        let compiler = Compiler::new(&robot);
        let cylon = compiler.compile(file.as_ref()).await.unwrap();

        Self {
            robot,
            compiled_data: cylon,
        }
    }

    pub fn is_allowed(&self, url: &Url) -> bool {
        self.compiled_data.allow(url.path())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_robots_map() {
        let mut map = RobotsMap::default();
        assert_eq!(
            map.is_allowed(
                "UndefinedRobot",
                Url::parse("https://www.yandex.com/images/123").unwrap(),
            )
            .await
            .unwrap(),
            false
        );
        assert_eq!(
            map.is_allowed(
                "Twitterbot",
                Url::parse("https://www.yandex.com/images/123").unwrap(),
            )
            .await
            .unwrap(),
            true
        );
    }
}
