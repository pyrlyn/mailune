import org.gradle.api.tasks.SourceSetContainer
import org.gradle.api.tasks.compile.JavaCompile
import org.gradle.api.tasks.testing.Test
import org.gradle.api.tasks.testing.logging.TestExceptionFormat

// The Kotlin Gradle plugin marker is not in the local cache. Compile with the
// compiler jars that ship inside the Gradle installation, and stay offline.

apply(plugin = "java")

val sourceSets = extensions.getByType(SourceSetContainer::class.java)
val gradleHome = gradle.gradleHomeDir ?: error("Gradle home is missing")
val gradleLib = gradleHome.resolve("lib")

fun libJar(prefix: String): File {
    val matches = gradleLib.listFiles { file: File ->
        file.isFile && file.name.startsWith(prefix) && file.name.endsWith(".jar")
    } ?: emptyArray()
    return matches.singleOrNull() ?: error("Gradle installation is missing $prefix")
}

val kotlinStdlib: File = libJar("kotlin-stdlib-")
val compilerClasspath = files(
    libJar("kotlin-compiler-embeddable-"),
    kotlinStdlib,
    libJar("kotlin-reflect-"),
    libJar("kotlinx-coroutines-core-jvm-"),
    libJar("kotlin-script-runtime-"),
    libJar("annotations-"),
)

fun registerKotlinCompile(
    taskName: String,
    sourceDir: String,
    outputPath: String,
    compileClasspath: FileCollection,
): TaskProvider<JavaExec> {
    return tasks.register(taskName, JavaExec::class.java) {
        val sources = fileTree(sourceDir) { include("**/*.kt") }
        classpath = compilerClasspath
        mainClass.set("org.jetbrains.kotlin.cli.jvm.K2JVMCompiler")
        val output = layout.buildDirectory.dir(outputPath)
        inputs.files(sources)
        inputs.files(compileClasspath)
        outputs.dir(output)
        onlyIf { sources.files.isNotEmpty() }
        doFirst {
            val outDir = output.get().asFile
            outDir.mkdirs()
            val arguments = mutableListOf(
                "-no-stdlib",
                "-no-reflect",
                "-jvm-target",
                "21",
                "-d",
                outDir.absolutePath,
                "-classpath",
                compileClasspath.files.filter { it.exists() }.joinToString(File.pathSeparator) { it.absolutePath },
            )
            sources.files.sortedBy { it.path }.forEach { arguments.add(it.absolutePath) }
            setArgs(arguments)
        }
    }
}

val mainKotlinOut = "classes/kotlin/main"
val testKotlinOut = "classes/kotlin/test"
val compileKotlin = registerKotlinCompile(
    "compileKotlin",
    "src/main/kotlin",
    mainKotlinOut,
    files(kotlinStdlib, sourceSets.getByName("main").compileClasspath),
)
compileKotlin.configure { dependsOn(tasks.named("compileJava")) }

val testJavaClasses = tasks.named("compileTestJava", JavaCompile::class.java).flatMap { it.destinationDirectory }
val compileTestKotlin = registerKotlinCompile(
    "compileTestKotlin",
    "src/test/kotlin",
    testKotlinOut,
    files(kotlinStdlib, sourceSets.getByName("main").output, testJavaClasses),
)
compileTestKotlin.configure { dependsOn(compileKotlin, tasks.named("compileTestJava")) }

sourceSets.getByName("main").output.dir(
    mapOf("builtBy" to compileKotlin),
    layout.buildDirectory.dir(mainKotlinOut),
)
val testKotlinSources = fileTree("src/test/kotlin") { include("**/*.kt") }
if (testKotlinSources.files.isNotEmpty()) {
    sourceSets.getByName("test").java.srcDir(rootDir.resolve("gradle/junit-stub"))
}
sourceSets.getByName("test").output.dir(
    mapOf("builtBy" to compileTestKotlin),
    layout.buildDirectory.dir(testKotlinOut),
)

dependencies.add("runtimeOnly", files(kotlinStdlib))

tasks.named("test", Test::class.java) {
    useJUnit()
    setTestClassesDirs(
        files(
            layout.buildDirectory.dir("classes/java/test"),
            layout.buildDirectory.dir(testKotlinOut),
        ),
    )
    dependsOn(compileTestKotlin)
    testLogging {
        events("passed", "failed", "skipped")
        exceptionFormat = TestExceptionFormat.FULL
    }
}
