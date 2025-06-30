package poc.p2p.app.ui.theme
import androidx.compose.runtime.Composable
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import poc.p2p.app.ui.MainScreen


@Composable
fun AppNav() {
    val navController = rememberNavController()
    NavHost(navController, startDestination = "main") {
        composable("main") { MainScreen(navController) }
        composable("topics") { TopicsScreen() }
        composable("reputations") { ReputationsScreen() }
    }
}