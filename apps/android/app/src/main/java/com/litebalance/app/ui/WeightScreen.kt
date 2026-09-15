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
import java.time.LocalDateTime
import java.time.format.DateTimeFormatter

@Composable
fun WeightScreen() {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var weight by remember { mutableStateOf("63.0") }
    var history by remember { mutableStateOf("") }

    fun reload() {
        scope.launch {
            runCatching {
                withContext(Dispatchers.IO) {
                    val session = CoreSession.get(context)
                    val user = demoUserInput()
                    session.upsertUser(user)
                    session.weightHistory(user.userId, 30u)
                        .joinToString("\n") { "${it.date}: ${"%.2f".format(it.weightKg)} kg" }
                        .ifBlank { "暂无记录" }
                }
            }.onSuccess { history = it }
                .onFailure { history = "错误：${it.message}" }
        }
    }

    ScreenColumn {
        Text("体重", style = androidx.compose.material3.MaterialTheme.typography.headlineSmall)
        LabeledField("体重 kg", weight) { weight = it }
        Button(onClick = {
            scope.launch {
                runCatching {
                    withContext(Dispatchers.IO) {
                        val session = CoreSession.get(context)
                        val user = demoUserInput()
                        session.upsertUser(user)
                        val stamp = LocalDateTime.now().format(DateTimeFormatter.ofPattern("yyyy-MM-dd'T'HH:mm:ss"))
                        session.logWeight(user.userId, weight.toDouble(), null, stamp)
                    }
                }.onSuccess { reload() }
                    .onFailure { history = "错误：${it.message}" }
            }
        }) { Text("记录体重") }
        Button(onClick = { reload() }) { Text("刷新历史") }
        SectionCard("历史", history.ifBlank { "点击刷新" })
    }
}
