package poc.p2p.app.ui.theme
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp


@Composable
fun ReputationsScreen() {
    Column(modifier = Modifier.padding(16.dp)) {
        Text("Reputations", style = MaterialTheme.typography.headlineSmall)
    }
}