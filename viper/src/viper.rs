// © 2019, ETH Zurich
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use error_chain::ChainedError;
use jni::*;
use jni_utils::JniUtils;
use std::env;
use std::fs;
use verification_context::*;
use viper_sys::wrappers::*;

pub struct Viper {
    jvm: JavaVM,
}

impl Default for Viper {
    fn default() -> Self {
        Self::new()
    }
}

impl Viper {
    pub fn new() -> Self {
        Self::new_with_args(vec![], "silicon")
    }

    pub fn new_with_args(java_args: Vec<String>, viper_backend: &str) -> Self {
        let viper_home = env::var("VIPER_HOME").unwrap_or_else(|_| "/usr/lib/viper/".to_string());
        let heap_size = env::var("JAVA_HEAP_SIZE").unwrap_or_else(|_| "4096".to_string());

        debug!("Using Viper home: '{}'", &viper_home);

        let jar_paths: Vec<String> = fs::read_dir(viper_home)
            .unwrap()
            .map(|x| x.unwrap().path().to_str().unwrap().to_string())
            .filter(|path| {
                if viper_backend == "carbon" {
                    !path.contains("silicon")
                } else if viper_backend == "silicon" {
                    !path.contains("carbon")
                } else {
                    true
                }
            })
            .collect();

        debug!("Java classpath: {}", jar_paths.clone().join(":"));

        let classpath_separator = if cfg!(windows) { ";" } else { ":" };
        let init_args = InitArgsBuilder::new()
            .version(JNIVersion::V8)
            .option(&format!("-Djava.class.path={}", jar_paths.join(classpath_separator)))
            // maximum heap size
            .option(&format!("-Xmx{}m", heap_size))
            // stack size
            .option("-Xss1024m");
            //.option("-Xdebug")
            //.option("-verbose:gc")
            //.option("-Xcheck:jni")
            //.option("-XX:+CheckJNICalls")
            //.option("-Djava.security.debug=all")
            //.option("-verbose:jni")
            //.option("-XX:+TraceJNICalls")
        let jvm_args = java_args
            .into_iter()
            .fold(init_args, |curr_args, opt| curr_args.option(&opt))
            .build()
            .unwrap_or_else(|e| {
                panic!(e.display_chain().to_string());
            });

        let jvm = JavaVM::new(jvm_args).unwrap_or_else(|e| {
            panic!(e.display_chain().to_string());
        });

        // Log JVM and Java version
        {
            let env = jvm
                .attach_current_thread()
                .expect("failed to attach jvm thread");

            let jni = JniUtils::new(&env);

            let system_wrapper = java::lang::System::with(&env);

            let vm_name = jni.to_string(
                jni.unwrap_result(system_wrapper.call_getProperty(jni.new_string("java.vm.name"))),
            );

            let java_version = jni.to_string(
                jni.unwrap_result(system_wrapper.call_getProperty(jni.new_string("java.version"))),
            );

            info!("Using JVM {}, Java {}", vm_name, java_version);
        }

        let this = Viper { jvm };

        this
    }

    pub fn new_verification_context(&self) -> VerificationContext {
        let env_guard = self
            .jvm
            .attach_current_thread()
            .expect("failed to attach jvm thread");

        VerificationContext::new(env_guard)
    }
}
