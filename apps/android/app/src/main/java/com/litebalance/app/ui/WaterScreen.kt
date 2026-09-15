package com.litebalance.app.ui

import androidx.compose.material3.Button
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import com.litebalance.app.CoreSession
import com.litebalance.app.demoUserInput
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.time.LocalDate
import java.time.LocalDateTime
import java.time.format.DateTimeFormatter

@Composable
fun WaterScreen() {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var amount by remember { mutableStateOf("250") }
    var summary by remember { mutableStateOf("") }

    fun reload() {
        scope.launch {
            runCatching {
                withContext(Dispatchers.IO) {
                    val session = CoreSession.get(context)
                    val user = demoUserInput()
                    session.upsertUser(user)
                    val today = LocalDate.now().toString()
                    val s = session.dailyWater(user.userId, today, 0u, user)
                    "目标 ${s.goalMl} ml\n已喝 ${s.totalConsumedMl} ml\n进度 ${"%.1f".format(s.progressPct)}%\n剩余 ${s.remainingMl} ml"
                }
            }.onSuccess { summary = it }
                .onFailure { summary = "错误：${it.message}" }
        }
    }

    ScreenColumn {
        Text("饮水", style = androidx.compose.material3.MaterialTheme.typography.headlineSmall)
        LabeledField("本次 ml", amount) { amount = it }
        Button(onClick = {
            scope.launch {
                runCatching {
                    withContext(Dispatchers.IO) {
                        val session = CoreSession.get(context)
                        val user = demoUserInput()
                        session.upsertUser(user)
                        val stamp = LocalDateTime.now().format(DateTimeFormatter.ofPattern("yyyy-MM-dd'T'HH:mm:ss"))
                        session.logWater(user.userId, amount.toUInt(), stamp)
                    }
                }.onSuccess { reload() }
                    .onFailure { summary = "错误：${it.message}" }
            }
        }) { Text("记录饮水") }
        Button(onClick = { reload() }) { Text("刷新今日进度") }
        SectionCard("今日", summary.ifBlank { "点击刷新" })
    }
}
