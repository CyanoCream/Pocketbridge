package com.mks.pocketbridge

import java.net.Inet4Address
import java.net.NetworkInterface

object NetUtil {
    // Cellular and virtual interfaces are not reachable from the LAN.
    private val SKIP = listOf("rmnet", "ccmni", "dummy", "tun", "v4-", "p2p", "ipsec", "clat")

    /**
     * (interface, ip) for every LAN-reachable IPv4 — WiFi client (wlan*), hotspot (ap*, swlan*), ethernet.
     * Any address range is accepted: some routers hand out non-private ranges (e.g. 50.50.50.x).
     */
    fun lanAddresses(): List<Pair<String, String>> {
        val res = ArrayList<Pair<String, String>>()
        try {
            for (ni in NetworkInterface.getNetworkInterfaces()?.toList().orEmpty()) {
                if (!ni.isUp || ni.isLoopback) continue
                val n = ni.name.lowercase()
                if (SKIP.any { n.startsWith(it) }) continue
                for (a in ni.inetAddresses.toList()) {
                    if (a is Inet4Address && !a.isLoopbackAddress && !a.isLinkLocalAddress) {
                        res += ni.name to (a.hostAddress ?: continue)
                    }
                }
            }
        } catch (_: Exception) {
        }
        return res.sortedBy { if (it.first.startsWith("wlan")) 0 else 1 }
    }
}
