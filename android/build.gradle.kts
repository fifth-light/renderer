import com.nishtahir.CargoBuildTask
import org.gradle.internal.extensions.stdlib.capitalized

plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.jetbrains.kotlin.android)
    alias(libs.plugins.rust.android.gradle)
}

android {
    namespace = "top.fifthlight.renderer"
    compileSdk = 35
    ndkVersion = "28.0.12433566"

    defaultConfig {
        applicationId = "top.fifthlight.renderer"
        minSdk = 26
        targetSdk = 35
        versionCode = 1
        versionName = "1.0"
    }

    buildTypes {
        release {
            isMinifyEnabled = true
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
}

cargo {
    module = "../crates/renderer-android"
    targetDirectory = "../target"
    libname = "renderer_android"
    targets = listOf("arm64")
    profile = "release"
    features {
        defaultAnd(arrayOf("log-panics"))
    }
}

project.afterEvaluate {
    android.applicationVariants.forEach { variant ->
        tasks.getByName("merge${variant.name.capitalized()}JniLibFolders") {
            dependsOn(tasks.withType(CargoBuildTask::class.java))
        }
    }
}

dependencies {
    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.appcompat)
    implementation(libs.androidx.games.activity)
}