package poc.p2p.app

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent

import android.widget.*
import androidx.compose.material3.MaterialTheme
import poc.p2p.app.ui.MainScreen
import poc.p2p.app.ui.theme.AppNav
import uniffi.bindings_p2p.*  // <- generado por uniffi-bindgen

//class MainActivity : AppCompatActivity() {
class MainActivity : ComponentActivity() {

    companion object {
        init {
            System.loadLibrary("uniffi_bindings_p2p")
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        setContent {
            MaterialTheme {
                AppNav()
            }
        }
        /*
        setContent {
            MaterialTheme {
                MainScreen()
            }
        }

         */
    }


    /*
    private val messages = CopyOnWriteArrayList<String>()

    private lateinit var textView: TextView
    private lateinit var input: EditText
    private lateinit var sendButton: Button
    private lateinit var connectButton: Button



    private lateinit var serverAddressInput: EditText
    private lateinit var publicKeyInput: EditText
    private lateinit var usernameInput: EditText

    private fun handleIncomingLink(intent: Intent?) {
        if (intent?.action == Intent.ACTION_SEND && intent.type == "text/plain") {
            val sharedText = intent.getStringExtra(Intent.EXTRA_TEXT)
            if (!sharedText.isNullOrEmpty()) {
                Log.d("Compartido", "Texto recibido: $sharedText")
            }
        }
    }


    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        handleIncomingLink(intent)
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        handleIncomingLink(intent);


        // Carga el layout base
        setContentView(buildUI())

        // Listener para recibir mensajes desde Rust
        val listener = object : EventListener {
            override fun onEvent(event: Event): String {
                runOnUiThread {
                    messages.add(event.topic + ":" + event.message)
                    textView.text = messages.joinToString("\n")
                }
                return ""
            }
        }

        // Enlaza el listener
        dummySetListener(listener)

        connectButton.setOnClickListener {
            val server = serverAddressInput.text.toString()
            val publicKey = publicKeyInput.text.toString()
            val username = usernameInput.text.toString()

            if (server.isNotBlank() && publicKey.isNotBlank() && username.isNotBlank()) {
                dummyStart(server, publicKey, username)
            }
        }


        sendButton.setOnClickListener {
            val msg = input.text.toString()


            if (msg.isNotBlank()) {
                dummyRawMessage("chat-room", msg)
                input.setText("")
            }
        }
    }

    private fun buildUI(): LinearLayout {
        val layout = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(16, 16, 16, 16)
        }

        serverAddressInput = EditText(this).apply {
            hint = "Server address (e.g., 1.2.3.4:9876)"
            setText("/ip4/10.0.2.2/tcp/34291")
        }

        publicKeyInput = EditText(this).apply {
            hint = "Peer id (hex/base64)"
            setText("12D3KooWGL8UXykLtTBpJokYemExXpybATqFRiJVcESpaY1dgZvx")
        }

        usernameInput = EditText(this).apply {
            hint = "Username"
            setText("Testuser")
        }

        connectButton = Button(this).apply {
            text = "Connect"
        }

        textView = TextView(this).apply {
            textSize = 16f
        }

        input = EditText(this).apply {
            hint = "Type a message..."
            setText(java.util.UUID.randomUUID().toString())
        }

        sendButton = Button(this).apply {
            text = "Start and Send"
        }

        layout.addView(serverAddressInput)
        layout.addView(publicKeyInput)
        layout.addView(usernameInput)
        layout.addView(connectButton)
        layout.addView(textView)
        layout.addView(input)
        layout.addView(sendButton)

        return layout
    }

     */

}
