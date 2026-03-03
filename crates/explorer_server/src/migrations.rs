enum Versions {
    V0,
    V1,
    V2,
    V3,
}

trait Migration {
    fn version(&self) -> Versions;
    async fn migrate(&self) -> ();
}