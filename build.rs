extern crate bindgen;

use std::path;
use std::fs;
use std::ffi;
use std::io::Write;


const BREAK_ON_CAP: bool = false;


fn delete_dir_contents(
    dir: &path::PathBuf,
    while_list: &Vec<path::PathBuf>,
    log_file: &mut fs::File)
{
    let paths : Vec<path::PathBuf> = {
        let mut p = Vec::new();
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            p.push(entry.path());
        }
        p
    };

    for path in paths {
        if while_list.contains(&path) {
            writeln!(log_file, "Skipping file: {:?}", path).unwrap();
        } else {
            if path.is_dir() {
                writeln!(log_file, "Deleting dir: {:?}", path).unwrap();
                fs::remove_dir_all(path).unwrap();
            } else {
                writeln!(log_file, "Deleting file: {:?}", path).unwrap();
                fs::remove_file(path).unwrap();
            }
        }
    }
}

fn get_files_paths(dir: &path::Path, log_file: &mut fs::File) -> Vec<path::PathBuf> {
    let mut paths : Vec<path::PathBuf> = Vec::new();

    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            let mut sub_paths = get_files_paths(&path, log_file);
            paths.append(&mut sub_paths);
        } else {
            writeln!(log_file, "Found file: {:?}", path).unwrap();
            paths.push(path);
        }
    }

    return paths;
}


#[derive(Debug)]
struct FileConversion<'a> {
    /// base directory for import
    pub souce_base_dir: path::PathBuf,
    /// relative path for source file
    pub source_rel_path: path::PathBuf,

    /// base directory for destination
    pub dest_base_dir: path::PathBuf,
    /// relative path to destination file
    pub dest_rel_path: path::PathBuf,

    // log file
    pub log_file: &'a fs::File,
    pub hidden_regex_string: String
}

fn get_parent_module_path<P: AsRef<path::Path>>(dir_path: P) -> path::PathBuf {
    // the parent directory path and on OsString of the parent 
    // directory's name
    let parent_dir_path = dir_path.as_ref().parent().unwrap();
    let parent_dir_name = match parent_dir_path.file_name() {
        Some(s) => s,
        None => ffi::OsStr::new("")
    };
    // path to where to declare new module
    let parent_mod_path = match parent_dir_name.to_str().unwrap() {
        "src" => parent_dir_path.join("lib.rs"),
        _ => parent_dir_path.join("mod.rs")
    };
    return parent_mod_path;
}

fn sanatize_rust_module_name<P: AsRef<path::Path>>(p: P) -> path::PathBuf {
    let mut p_string = p.as_ref().to_path_buf().into_os_string().into_string().unwrap();
    p_string = p_string.replace("-","_");
    path::PathBuf::from(p_string)
}

impl<'a> FileConversion<'a> {

    #[allow(dead_code)]
    fn log_self(&mut self) {
        let s = format!("FileCoversion: {:?}", self);
        writeln!(self.log_file, "{}", s).unwrap();
    }

    fn rel_dest_dir(&self) -> path::PathBuf {
        self.dest_rel_path.parent().unwrap().to_path_buf()
    }

    fn create_mod_at_dir<P: AsRef<path::Path>>(&mut self, dir_path_: P) {

        let dir_path = dir_path_.as_ref();
        //writeln!(self.log_file, "Attempting create new module: {:?}", dir_path).unwrap();

        // path to where to declare new module
        let parent_mod_path = get_parent_module_path(&dir_path);
        // name of the new module
        let mod_name = dir_path.file_name().unwrap().to_str().unwrap();
        // path to mod file inside dir_path
        let dir_mod_file_path = dir_path.join("mod.rs");

        match (dir_path.is_dir(), dir_path.is_file()) {
            // if the dir path definitely doesn't exist
            (false,false) => {
                writeln!(self.log_file, "Creating new directory: {:?}", dir_path).unwrap();
                fs::create_dir(&dir_path).unwrap();

                writeln!(self.log_file, "Creating new file mod.rs: {:?}", dir_mod_file_path).unwrap();
                fs::File::create(&dir_mod_file_path).unwrap();

                writeln!(self.log_file, "Declaring new module in parent module: {:?}", parent_mod_path).unwrap();
                let mut parent_mod_file = fs::OpenOptions::new()
                    .create(false).append(true).write(true)
                    .open(parent_mod_path).unwrap();
                write!(parent_mod_file, "\npub mod {};\n", mod_name).unwrap();
            },
            // if it exists as a directory
            (true,false) => (), //writeln!(self.log_file, "Directory already exists: {:?}", dir_path).unwrap(),
            // if it exists as a file
            _ => unreachable!("Shouldn't have a module path be a file!")
        }
    }

    fn create_dest_dirs(&mut self) {
        let mut path_accumulator = self.dest_base_dir.clone();
        for component in self.rel_dest_dir().components() {
            path_accumulator.push(component.as_ref());
            self.create_mod_at_dir(&path_accumulator);
        }
    }

    fn attempt_create_header_module(&mut self) -> Result<(),String> {

        let dest_path = self.dest_base_dir.join(&self.dest_rel_path);

        let include_str_path = self.source_rel_path.to_str().unwrap();
        let include_str = format!("#include <{}>", &include_str_path);

        writeln!(self.log_file, "Attempting build bindgen for '{}': {}", include_str_path, include_str).unwrap();

        let bindings_result = bindgen::Builder::default()
            .header_contents(include_str_path, &include_str)
            .hide_type(&self.hidden_regex_string)
            .generate();
        
        let bindings = match bindings_result {
            Ok(b) => b,
            Err(e) => {
                let s = format!("Failed to generate bindings for '{}', error: {:?}", include_str_path, e);
                writeln!(self.log_file, "{}", s).unwrap();
                fs::File::create(&dest_path).unwrap();
                return Err(s);
            }
        };

        writeln!(self.log_file, "Attempting write generate code to: {:?}", &dest_path).unwrap();
        bindings
            .write_to_file(&dest_path)
            .expect(&format!("Couldn't write bindings {:?}!", dest_path));


        let parent_mod_path = get_parent_module_path(&dest_path);
        writeln!(self.log_file, "Attempting to add new module declaration to: {:?}", &parent_mod_path).unwrap();

        let mut parent_mod_file = fs::OpenOptions::new()
            .create(false).append(true).write(true)
            .open(parent_mod_path).unwrap();

        let file_stem = dest_path.file_stem().unwrap();
        let file_stem_str = file_stem.to_str().unwrap();
        writeln!(parent_mod_file, "\npub mod {};\n", file_stem_str).unwrap();
        Ok(())
    }

}

fn main() {
    // constants
    let src_dir = path::PathBuf::from("./src/");
    let linux_include_uapi_path = path::PathBuf::from("./linux/include/uapi/");
    let log_file_path = path::PathBuf::from("./build_rs.log");
    let src_lib_file_path = src_dir.join("lib.rs");
    let new_src_lib_file_path = src_dir.join("TEMP_lib.rs");

    let hidden_types = vec![
        //"atm_kptr_t",
    ];

    
    let mut hidden_regex_string = String::from("^(");
    for entry in hidden_types {
        hidden_regex_string.push_str(entry);
        hidden_regex_string.push('|');
    }
    let _ = hidden_regex_string.pop().unwrap();
    hidden_regex_string.push_str(")$");



    // setup logging
    let _ = fs::remove_file(&log_file_path);
    let mut log_file = fs::File::create(&log_file_path).unwrap();

    writeln!(log_file, "Hidden type regex:\n{}", hidden_regex_string).unwrap();


    // remove prior generated files
    writeln!(log_file, "Removing all contents of: {:?}", src_dir).unwrap();
    let white_list = vec![src_lib_file_path.clone(), src_dir.join(".gitkeep")];
    delete_dir_contents(&src_dir, &white_list, &mut log_file);

    // zero out src/lib.rs
    writeln!(log_file, "Creating blank file: {:?}", src_lib_file_path).unwrap();
    fs::File::create(&new_src_lib_file_path).unwrap();
    writeln!(log_file, "Replacing {:?} with new blank file.", src_lib_file_path).unwrap();
    fs::rename(&new_src_lib_file_path, &src_lib_file_path).unwrap();

    let mut failed_bindings : Vec<path::PathBuf> = Vec::new();

    let mut counter: u64 = 0;
    let cap = 100;

    for header in get_files_paths(&linux_include_uapi_path, &mut log_file) {

        if counter >= cap && BREAK_ON_CAP { break; }
        counter += 1;

        if header.extension() != Some(ffi::OsStr::new("h")) {
            writeln!(log_file, "Skipping non-header file: {:?}", header).unwrap();
            continue;
        }
        writeln!(log_file, "Processing header: {:?}", header).unwrap();

        let source_rel_path = header.strip_prefix(&linux_include_uapi_path).unwrap().to_path_buf();

        let mut fc = FileConversion {
            souce_base_dir: linux_include_uapi_path.clone(),
            source_rel_path: source_rel_path.clone(),
            dest_base_dir: src_dir.clone(),
            dest_rel_path: {
                let mut ret_val = source_rel_path.clone();
                ret_val.set_extension("rs");
                sanatize_rust_module_name(&ret_val)
            },
            log_file: &log_file,
            hidden_regex_string: hidden_regex_string.clone()
        };

        //fc.log_self();
        fc.create_dest_dirs();
        match fc.attempt_create_header_module() {
            Err(_) => failed_bindings.push(source_rel_path.clone()),
            _ => ()
        }
    }


    writeln!(
        log_file, "\nThe following {} bindings failed (of {} total):",
        failed_bindings.len(), counter
    ).unwrap();

    for binding in failed_bindings {
        writeln!(log_file, "\t{:?}", binding).unwrap();
    }

}
