plugins {
    kotlin("jvm") version "1.6.10"
    kotlin("plugin.serialization") version "1.6.10"
    id("java")
    idea
    `maven-publish`
    `java-library`
    signing
    id("io.github.gradle-nexus.publish-plugin") version "1.1.0"
}

group = "io.provenance.bilateral"
version = project.property("version")?.takeIf { it != "unspecified" }?.toString() ?: "1.0-SNAPSHOT"

repositories {
    mavenLocal()
    mavenCentral()
}

dependencies {
    listOf(
        libs.bouncyCastle,
        libs.jacksonDataTypeJdk8,
        libs.jacksonDataTypeJsr310,
        libs.jacksonDataTypeProtobuf,
        libs.jacksonModuleKotlin,
        libs.kotlinStdlib,
        libs.protobuf,
        libs.protobufUtil,
        libs.provenanceGrpcClient,
        libs.provenanceHdWallet,
        libs.provenanceProtoKotlin,
        libs.provenanceScopeEncryption,
        libs.provenanceScopeUtil,
    ).forEach(::api)

    listOf(
        libs.assetSpec
    ).forEach(::testImplementation)
}


tasks.withType<org.jetbrains.kotlin.gradle.tasks.KotlinCompile> {
    sourceCompatibility = "11"
    targetCompatibility = "11"

    kotlinOptions {
        freeCompilerArgs = listOf("-Xjsr305=strict")
        jvmTarget = "11"
    }
}

tasks.withType<JavaCompile> {
    sourceCompatibility = JavaVersion.VERSION_11.toString()
    targetCompatibility = JavaVersion.VERSION_11.toString()
}

configure<JavaPluginExtension> {
    sourceCompatibility = JavaVersion.VERSION_11
    targetCompatibility = JavaVersion.VERSION_11
}

configure<io.github.gradlenexus.publishplugin.NexusPublishExtension> {
    repositories {
        sonatype {
            nexusUrl.set(uri("https://s01.oss.sonatype.org/service/local/"))
            snapshotRepositoryUrl.set(uri("https://s01.oss.sonatype.org/content/repositories/snapshots/"))
            username.set(findProject("ossrhUsername")?.toString() ?: System.getenv("OSSRH_USERNAME"))
            password.set(findProject("ossrhPassword")?.toString() ?: System.getenv("OSSRH_PASSWORD"))
            stagingProfileId.set("3180ca260b82a7") // prevents querying for the staging profile id, performance optimization
        }
    }
}

java {
    withSourcesJar()
    withJavadocJar()
}

val artifactName = "bilateral-client"
val artifactVersion = version.toString()

configure<PublishingExtension> {
    publications {
        create<MavenPublication>("maven") {
            groupId = "io.provenance.bilateral"
            artifactId = artifactName
            version = artifactVersion

            from(components["java"])

            pom {
                name.set("Provenance Blockchain Metadata Bilateral Exchange Kotlin Client")
                description.set("A client to make GRPC requests to the Metadata Bilateral Exchange smart contract")
                url.set("https://provenance.io")
                licenses {
                    license {
                        name.set("The Apache License, Version 2.0")
                        url.set("http://www.apache.org/licenses/LICENSE-2.0.txt")
                    }
                }
                developers {
                    developer {
                        id.set("hyperschwartz")
                        name.set("Jacob Schwartz")
                        email.set("jschwartz@figure.com")
                    }
                }
                scm {
                    developerConnection.set("git@github.com:provenance.io/metadata-bilateral-exchange.git")
                    connection.set("https://github.com/provenance-io/metadata-bilateral-exchange.git")
                    url.set("https://github.com/provenance-io/metadata-bilateral-exchange")
                }
            }
        }

        configure<SigningExtension> {
            sign(publications["maven"])
        }
    }
}
