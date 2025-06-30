package poc.p2p.app.ui

import androidx.compose.foundation.border
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.navigation.NavHostController

@Composable
fun MainScreen(navController: NavHostController) {
    var serverId by remember { mutableStateOf("") }
    var serverAddress by remember { mutableStateOf("") }
    val pendingItems = listOf("row_1", "row_2", "row_3", "row_4")

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        Text("APP:", fontWeight = FontWeight.Bold)

        OutlinedTextField(
            value = serverId,
            onValueChange = { serverId = it },
            label = { Text("Server ID") },
            modifier = Modifier.fillMaxWidth()
        )

        OutlinedTextField(
            value = serverAddress,
            onValueChange = { serverAddress = it },
            label = { Text("Server Address") },
            modifier = Modifier.fillMaxWidth()
        )

        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            Button(onClick = { println("CONNECT $serverId $serverAddress") },
                modifier = Modifier
                    .weight(1f)
                    .fillMaxWidth()) {
                Text("CONNECT")
            }
            Button(onClick = { println("MAGIC PRESSED") },
                modifier = Modifier
                    .weight(1f)
                    .fillMaxWidth()) {
                Text("MAGIC")
            }
        }

        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(8.dp)
        ){
            Button(onClick = {  navController.navigate("reputations")  },
                modifier = Modifier
                    .weight(1f)
                    .fillMaxWidth()) {
                Text("REPUTATION")
            }
            Button(onClick = {   navController.navigate("topics") },
                modifier = Modifier
                    .weight(1f)
                    .fillMaxWidth()) {
                Text("TOPICS")
            }
        }

        Divider()
        Text("VALIDATED CONTENT", fontWeight = FontWeight.Bold)

        Spacer(modifier = Modifier.height(8.dp))

        Text("PENDING:", fontWeight = FontWeight.Bold)
        LazyColumn(
            modifier = Modifier
                .fillMaxHeight()
                .fillMaxWidth()
                .border(1.dp, Color.Gray)
                .padding(8.dp)
        ) {
            items(pendingItems) { item ->
                Text(item)
            }
        }
    }
}
