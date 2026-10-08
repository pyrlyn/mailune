apply(from = rootDir.resolve("gradle/kotlin-jvm.gradle.kts"))

dependencies.add("implementation", dependencyFactory.createProjectDependency(":core"))
dependencies.add("implementation", dependencyFactory.createProjectDependency(":ui"))
