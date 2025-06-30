package poc.p2p.app

import uniffi.bindings_p2p.Reputation
import uniffi.bindings_p2p.Votation
import java.util.Arrays

object P2PService {

    suspend fun magic_connection(): Pair<String, String> {
        return Pair("jola", "jola")
    }

    suspend  fun start(server_address: String, server_id: String, username: String) {
        uniffi.bindings_p2p.start(server_address, server_id, username);
    }


    /* list reputations and topics */
    suspend fun reputations(topic: String): List<Reputation> {
        return uniffi.bindings_p2p.getReputations(topic)
    }
    suspend fun topics(): List<String>  {
        //check your topics
        return Arrays.asList<String>();

    }
    suspend fun new_topic(topic: String) {
        uniffi.bindings_p2p.remoteNewTopic(topic);
    }

    suspend fun register_topic(topic: String)  {
        uniffi.bindings_p2p.registerTopic(topic);
    }

    suspend fun add_vote() {
      //  uniffi.bindings_p2p.addVote()
    }


    /*  */
    suspend fun pending_to_evaluate(): List<Votation> {
        /*  are you stored these votations, if you are not part of the */
        return Arrays.asList<Votation>();
    }

    suspend fun add_content_to_validate(): List<String> {
//        uniffi.bindings_p2p.

    };

    suspend fun stop() {
    }
}
