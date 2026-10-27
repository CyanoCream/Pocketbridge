import org.jetbrains.kotlin.gradle.dsl.JvmTarget
import java.util.Properties

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "com.mks.pocketbridge"
    compileSdk = 36
    ndkVersion = "27.0.12077973"

    defaultConfig {
        applicationId = "com.mks.pocketbridge"
        minSdk = 26
        targetSdk = 36
        versionCode = 1
        versionName = "1.0.0"
    }

    val keystoreProps = rootProject.file("keystore.properties")
    signingConfigs {
        if (keystoreProps.exists()) {
            val p = Properties().apply { keystoreProps.inputStream().use { load(it) } }
            create("release") {
                storeFile = file(p.getProperty("storeFile"))
                storePassword = p.getProperty("storePassword")
                keyAlias = p.getProperty("keyAlias")
                keyPassword = p.getProperty("keyPassword")
            }
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
            signingConfig = signingConfigs.findByName("release")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

kotlin {
    compilerOptions {
        jvmTarget.set(JvmTarget.JVM_17)
    }
}

// Builds rust/ into app/src/main/jniLibs/<abi>/libpocketbridge.so before every Android build.
val cargoBin = File(System.getProperty("user.home"), ".cargo/bin")
val cargoBuild by tasks.registering(Exec::class) {
    workingDir = rootProject.file("rust")
    environment("ANDROID_NDK_HOME", android.ndkDirectory.absolutePath)
    environment("PATH", cargoBin.absolutePath + File.pathSeparator + System.getenv("PATH"))
    val exe = if (System.getProperty("os.name").startsWith("Windows")) "cargo.exe" else "cargo"
    commandLine(
        File(cargoBin, exe).absolutePath, "ndk",
        "-t", "arm64-v8a", "-t", "armeabi-v7a", "-t", "x86_64",
        "-P", "26",
        "-o", file("src/main/jniLibs").absolutePath,
        "build", "--release",
    )
    inputs.dir(rootProject.file("rust/src"))
    inputs.dir(rootProject.file("rust/web"))
    inputs.file(rootProject.file("rust/Cargo.toml"))
    inputs.file(rootProject.file("rust/.cargo/config.toml"))
    outputs.dir(file("src/main/jniLibs"))
}
tasks.named("preBuild") { dependsOn(cargoBuild) }
