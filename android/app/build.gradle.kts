import java.nio.file.Files
import java.nio.file.StandardCopyOption

plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.android)
    alias(libs.plugins.kotlin.compose)
    alias(libs.plugins.hilt)
    alias(libs.plugins.ksp)
}

android {
    namespace = "com.example.sabspicomatrix"
    compileSdk = 35

    defaultConfig {
        applicationId = "com.example.sabspicomatrix"
        minSdk = 29
        targetSdk = 34
        versionCode = 1
        versionName = "1.0"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }
    kotlinOptions {
        jvmTarget = "1.8"
    }
    buildFeatures {
        compose = true
    }
}

// ---- task: build Rust for each ABI, copy .so, generate UniFFI Kotlin ----
abstract class BuildRustAndUniFfi @Inject constructor(
    private val execOps: ExecOperations
) : DefaultTask() {

    @get:Input
    abstract val abis: ListProperty<String>

    /** Crate/lib base name (without the `lib` prefix). Hyphens will be normalised to underscores. */
    @get:Input
    abstract val libBaseName: Property<String>

    /** Android platform level to pass to cargo-ndk (use your minSdk) */
    @get:Input
    abstract val androidPlatform: Property<Int>

    /** Path to the Rust crate’s Cargo.toml */
    @get:InputFile
    abstract val cargoToml: RegularFileProperty

    // 🔑 Tell Gradle which files should trigger rerun
    @get:InputFiles
    @get:PathSensitive(PathSensitivity.RELATIVE)
    abstract val rustInputs: ConfigurableFileCollection

    @get:OutputDirectory
    abstract val uniffiOutDir: DirectoryProperty

    @get:OutputDirectory
    abstract val jniOutDir: DirectoryProperty

    @TaskAction
    fun buildAll() {
        val crateDir = cargoToml.get().asFile.parentFile
        val platform = androidPlatform.get().toString()
        val base = libBaseName.get().replace('-', '_') // lib file is lib<base>.so

        fun abiToTarget(abi: String) = when (abi) {
            "arm64-v8a" -> "aarch64-linux-android"
            "armeabi-v7a" -> "armv7-linux-androideabi"
            "x86_64" -> "x86_64-linux-android"
            else -> error("Unknown ABI $abi")
        }

        // 1) Build Rust per ABI (expects `cargo-ndk` on PATH)
        abis.get().forEach { abi ->
            val target = abiToTarget(abi)
            execOps.exec {
                workingDir = crateDir
                commandLine(
                    "cargo", "ndk",
                    "--platform", platform,
                    "--target", target,
                    "build", "--release"
                )
            }

            // 2) Copy lib into src/main/jniLibs/<abi>/lib<base>.so
            val produced = crateDir.toPath()
                .resolve("target/$target/release/lib$base.so")
                .toFile()

            val destDir = jniOutDir.get().asFile.resolve(abi).apply { mkdirs() }
            Files.copy(
                produced.toPath(), destDir.toPath().resolve("lib$base.so"),
                StandardCopyOption.REPLACE_EXISTING
            )
        }

        // 3) Generate UniFFI Kotlin
        uniffiOutDir.get().asFile.mkdirs()
        execOps.exec {
            workingDir = crateDir
            commandLine("cargo", "build", "--release")
        }

        execOps.exec {
            workingDir = crateDir
            commandLine(
                "cargo", "run", "--release", "--bin",
                "uniffi-bindgen", "generate",
                "--library", "target/release/lib$base.so",
                "--language", "kotlin",
                "--out-dir", uniffiOutDir.get().asFile.absolutePath,
            )
        }
    }
}

androidComponents {
    onVariants(selector().all()) { variant ->
        val cap = variant.name.replaceFirstChar { it.titlecase() }
        val crateRoot = layout.projectDirectory.dir("../../state-bindings")

        val task = tasks.register<BuildRustAndUniFfi>("buildRustAndUniFfi$cap") {
            // ABIs you ship
            abis.set(listOf("arm64-v8a", "armeabi-v7a", "x86_64"))

            // Your crate/lib name; if your crate is `state-bindings`,
            // the file is `libstate_bindings.so`, so set `state_bindings`.
            libBaseName.set("state_bindings")

            // Use the variant’s minSdk if available, else 21
            androidPlatform.set(variant.minSdk?.apiLevel ?: 21)

            // Adjust these two paths to your repo
            cargoToml.set(crateRoot.file("Cargo.toml"))

            // Inputs that should trigger rerun
            rustInputs.setFrom(
                crateRoot.asFileTree.matching {
                    include("**/*.rs", "**/*.toml", "Cargo.lock", "build.rs")
                    exclude("**/target/**")
                }
            )
        }

        // need to add the sources to java
        variant.sources.java?.addGeneratedSourceDirectory(task, BuildRustAndUniFfi::uniffiOutDir)

        // This wires task -> native packaging (no hard-coded task name)
        variant.sources.jniLibs?.addGeneratedSourceDirectory(task, BuildRustAndUniFfi::jniOutDir)
    }
}


dependencies {
    // For uniffi bindings
    implementation("net.java.dev.jna:jna:5.12.0@aar")

    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.lifecycle.runtime.ktx)
    implementation(libs.androidx.activity.compose)
    implementation(platform(libs.androidx.compose.bom))
    implementation(libs.androidx.ui)
    implementation(libs.androidx.ui.graphics)
    implementation(libs.androidx.ui.tooling.preview)
    implementation(libs.androidx.material3)
    testImplementation(libs.junit)
    androidTestImplementation(libs.androidx.junit)
    androidTestImplementation(libs.androidx.espresso.core)
    androidTestImplementation(platform(libs.androidx.compose.bom))
    androidTestImplementation(libs.androidx.ui.test.junit4)
    debugImplementation(libs.androidx.ui.tooling)
    debugImplementation(libs.androidx.ui.test.manifest)

    implementation(libs.hilt.android)
    ksp(libs.hilt.android.compiler)
    ksp(libs.hilt.compiler)
    ksp(libs.dagger.compiler)
    implementation(libs.androidx.hilt.navigation.compose)
    implementation(libs.androidx.viewmodel.compose)
    implementation(libs.androidx.lifecycle.runtime.compose)
}