// Matrix - Kod Yağmuru: Android projesinin Gradle ayarları.
// Proje tek parçadır; eklenti ve kitaplıklar Google ve Maven Central depolarından alınır.
pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "matrixcoderain"
