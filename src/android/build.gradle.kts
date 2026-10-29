// Matrix - Kod Yağmuru: Android ekran koruyucu uygulaması.
//
// Rust kitaplığını cargo-ndk ile derler, APK'ya ekler, yayın anahtarıyla
// imzalar ve son APK'yı depo kökündeki build/android klasörüne kopyalar. Kotlin desteği Android Gradle
// eklentisinin içindedir; ayrıca Kotlin eklentisi gerekmez.
import java.util.Properties
import javax.inject.Inject

plugins {
    id("com.android.application") version "9.4.1"
}

// Depo kökü; ortak build klasörü buradadır.
val repoRoot: File = rootDir.resolve("../..").canonicalFile

// Rust paketi; Cargo.toml bu klasördedir.
val rustCrate: File = repoRoot.resolve("src")

// Uygulama sürümü Cargo.toml dosyasından okunur; sürüm tek yerde tutulur.
val appVersion: String =
    Regex("""(?m)^version\s*=\s*"([^"]+)"""")
        .find(rustCrate.resolve("Cargo.toml").readText())
        ?.groupValues
        ?.get(1)
        ?: error("Cargo.toml içinde sürüm bulunamadı.")

// Gradle ara dosyaları da depo kökündeki build klasöründe tutulur.
layout.buildDirectory.set(repoRoot.resolve("build/gradle"))

// APK'ya girecek işlemci türleri: telefon için 64 bit ARM, bilgisayardaki
// Android öykünücüsü (emulator) için 64 bit x86. Rust kitaplığı ikisi için de derlenir.
val targetAbis = listOf("arm64-v8a", "x86_64")

// Yayın imzası bilgileri depo dışında, kullanıcı klasöründe tutulur:
// <kullanıcı>/.android/matrixcoderain-release.properties (storeFile, storePassword,
// keyAlias, keyPassword). Dosya yoksa APK hata ayıklama anahtarıyla imzalanır.
val releaseKeyFile = File(System.getProperty("user.home"), ".android/matrixcoderain-release.properties")
val releaseKey: Properties? =
    releaseKeyFile.takeIf { it.isFile }?.let { file ->
        Properties().apply { file.inputStream().use { load(it) } }
    }
if (releaseKey == null) {
    logger.warn("Yayın anahtarı bulunamadı ($releaseKeyFile); APK hata ayıklama anahtarıyla imzalanacak.")
}

android {
    namespace = "com.atifdag.matrixcoderain"
    compileSdk = 36
    // Rust kitaplığını derleyen NDK sürümü; değiştirilirse SDK'da kurulu olmalıdır.
    ndkVersion = "29.0.14206865"

    defaultConfig {
        applicationId = "com.atifdag.matrixcoderain"
        // java.time ve donanım hızlandırmalı çizim için en az Android 8.
        minSdk = 26
        targetSdk = 36
        versionCode = 1
        versionName = appVersion
        ndk {
            abiFilters += targetAbis
        }
    }

    signingConfigs {
        if (releaseKey != null) {
            create("release") {
                storeFile = file(releaseKey.getProperty("storeFile"))
                storePassword = releaseKey.getProperty("storePassword")
                keyAlias = releaseKey.getProperty("keyAlias")
                keyPassword = releaseKey.getProperty("keyPassword")
            }
        }
    }

    buildTypes {
        release {
            // Kullanılmayan kod ve kaynaklar atılır. Rust'a bağlanan işlevler ile
            // sistemin çağırdığı sınıflar varsayılan kurallarla korunur.
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"))
            signingConfig = signingConfigs.findByName("release") ?: signingConfigs.getByName("debug")
        }
    }
}

/** Rust kitaplığını Android için derleyen görev. Çıktı: <klasör>/<işlemci türü>/libmatrix_core.so */
abstract class CargoNdkTask : DefaultTask() {
    /** Derlenen kitaplıkların yazılacağı klasör. */
    @get:OutputDirectory
    abstract val outputDirectory: DirectoryProperty

    /** Derlenecek işlemci türleri (ör. arm64-v8a). */
    @get:Input
    abstract val abis: ListProperty<String>

    /** Android NDK klasörü; cargo-ndk derleyiciyi buradan bulur. */
    @get:Internal
    abstract val ndkDirectory: DirectoryProperty

    /** Cargo.toml dosyasının bulunduğu klasör; ara dosyalar burada target/ altına yazılır. */
    @get:Internal
    abstract val crateDirectory: DirectoryProperty

    @get:Inject
    abstract val execOperations: ExecOperations

    @TaskAction
    fun build() {
        execOperations.exec {
            workingDir = crateDirectory.get().asFile
            environment("ANDROID_NDK_HOME", ndkDirectory.get().asFile.absolutePath)
            val targets = abis.get().flatMap { listOf("-t", it) }
            commandLine(
                listOf("cargo", "ndk") + targets + listOf(
                    "-o", outputDirectory.get().asFile.absolutePath,
                    "build", "--release", "--locked",
                    "--lib", "--no-default-features", "--features", "ffi",
                ),
            )
        }
    }
}

val buildRustLibrary = tasks.register<CargoNdkTask>("buildRustLibrary") {
    outputDirectory.set(layout.buildDirectory.dir("rust"))
    abis.set(targetAbis)
    ndkDirectory.set(androidComponents.sdkComponents.ndkDirectory)
    crateDirectory.set(rustCrate)
    // Rust kaynakları Gradle'ın izlediği dosyalar değildir; değişiklik
    // denetimini cargo kendisi yapar. Bu yüzden görev her derlemede çalışır.
    outputs.upToDateWhen { false }
}

// Rust kitaplığı, APK'ya eklenecek yerel kitaplıklar arasına katılır.
androidComponents {
    onVariants { variant ->
        variant.sources.jniLibs?.addGeneratedSourceDirectory(
            buildRustLibrary,
            CargoNdkTask::outputDirectory,
        )
    }
}

// Son APK'yı depo kökündeki build/android klasörüne kopyalar.
tasks.register<Copy>("deliverApk") {
    dependsOn("assembleRelease")
    from(layout.buildDirectory.dir("outputs/apk/release")) {
        include("*.apk")
        rename { "matrixcoderain.apk" }
    }
    into(repoRoot.resolve("build/android"))
}
