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
import uniffi.litebalance.FfiGoalInput

@Composable
fun GoalScreen() {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var target by remember { mutableStateOf("60") }
    var rate by remember { mutableStateOf("-0.5") }
    var result by remember { mutableStateOf("") }

    ScreenColumn {
        Text("目标预算", style = androidx.compose.material3.MaterialTheme.typography.headlineSmall)
        LabeledField("目标体重 kg", target) { target = it }
        LabeledField("每周速率 kg（减重为负）", rate) { rate = it }
        Button(onClick = {
            scope.launch {
                result = "计算中…"
                runCatching {
                    withContext(Dispatchers.IO) {
                        val session = CoreSession.get(context)
                        val user = demoUserInput()
                        session.upsertUser(user)
                        session.setGoal(
                            user.userId,
                            FfiGoalInput(
                                kind = "lose",
                                targetWeightKg = target.toDouble(),
                                weeklyRateKg = rate.toDouble(),
                                taperEnabled = true,
                                adaptiveEnabled = true,
                                manualCalorieOffset = 0.0,
                            ),
                        )
                        val budget = session.computeBudget(user)
                        buildString {
                            append("基准 TDEE ${"%.0f".format(budget.baseTdeeKcal)} kcal\n")
                            append("每日预算 ${"%.0f".format(budget.dailyBudgetKcal)} kcal\n")
                            append("安全底线 ${"%.0f".format(budget.safetyFloorKcal)} kcal")
                            if (budget.safetyFloorTriggered) append("（已触发）")
                            append("\n宏量 P/C/F ${"%.0f".format(budget.recommendedProteinG)} / ")
                            append("${"%.0f".format(budget.recommendedCarbsG)} / ")
                            append("${"%.0f".format(budget.recommendedFatG)} g\n")
                            append(budget.clinicalNotes.joinToString("\n"))
                        }
                    }
                }.onSuccess { result = it }
                    .onFailure { result = "错误：${it.message}" }
            }
        }) { Text("保存目标并计算预算") }
        if (result.isNotBlank()) SectionCard("预算", result)
    }
}
