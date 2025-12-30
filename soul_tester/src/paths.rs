use std::io;
pub struct Paths {
    pub base: String,
    pub output: String,
    pub soul_src: String,
    pub output_ast: String,
}

impl Paths {
    pub fn new(raw_json: &[u8]) -> serde_json::Result<Self> {
        let JsonPath{path} = serde_json::from_slice(raw_json)?;

        let output_ast = format!("{path}/output/AST");
        let soul_src = format!("{path}/soul_src");
        let output = format!("{path}/output");
        let base = path;

        Ok(Paths{
            base,
            output,
            soul_src,
            output_ast,
        })
    }

    pub fn get_ast_incremental_ast(&self, file_name: &str) -> String {
        format!("{}/{file_name}.soulAST", self.output)
    } 

    pub fn get_ast_incremental_ast_meta(&self, file_name: &str) -> String {
        format!("{}/{file_name}.soulASTMeta", self.output)
    }

    pub fn insure_paths_exist(&self) -> io::Result<()> {
        std::fs::create_dir_all(&self.base)?;
        std::fs::create_dir_all(&self.output)?;
        std::fs::create_dir_all(&self.soul_src)?;
        std::fs::create_dir_all(&self.output_ast)
    }
}

#[derive(serde::Deserialize)]
struct JsonPath {path: String}