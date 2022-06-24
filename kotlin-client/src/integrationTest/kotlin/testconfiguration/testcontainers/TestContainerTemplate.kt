package testconfiguration.testcontainers

import org.testcontainers.containers.GenericContainer
import org.testcontainers.containers.Network

interface TestContainerTemplate<out T : GenericContainer<out T>> {
    val containerName: String

    fun buildContainer(network: Network): T
    fun afterStartup(container: @UnsafeVariance T) {}
    fun getTestProperties(container: @UnsafeVariance T): List<String> = emptyList()
}
