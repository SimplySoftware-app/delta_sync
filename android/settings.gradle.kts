pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "DeltaSync"
include(":app")

// Delta Sync core for Android (AAR): `./gradlew :sdk:assembleRelease`.
include(":sdk")
project(":sdk").projectDir = file("../sdk/kotlin")
