package com.litebalance.app

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.lifecycle.lifecycleScope
import com.litebalance.app.core.CoreSession
import com.litebalance.app.theme.LiteBalanceTheme
import com.litebalance.app.ui.home.HomeScreen
import com.litebalance.app.ui.navigation.AppDestination
import com.litebalance.app.ui.navigation.AppNavigationBar
import kotlinx.coroutines.launch

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)

        // 后台初始化沙箱 UniFFI 会话
        lifecycleScope.launch {
            CoreSession.initialize(applicationContext)
        }

        setContent {
            LiteBalanceTheme(darkTheme = false) {
                val snackbarHostState = remember { SnackbarHostState() }
                var currentDestination by remember { mutableStateOf(AppDestination.HOME) }

                Scaffold(
                    modifier = Modifier.fillMaxSize(),
                    containerColor = Color.White,
                    contentWindowInsets = WindowInsets(0, 0, 0, 0),
                    snackbarHost = { SnackbarHost(snackbarHostState) }
                ) { _ ->
                    Box(
                        modifier = Modifier.fillMaxSize()
                    ) {
                        // 直接点击切换页面（移除横向滑动手势）
                        when (currentDestination) {
                            AppDestination.HOME -> {
                                HomeScreen()
                            }
                            AppDestination.TRENDS -> {
                                DestinationPlaceholder(
                                    title = "趋势 (Kevin Hall 仿真 & 周期报表)",
                                    description = "基于 NIH Kevin Hall 动态体重常微分模型预测停滞期与真实体脂演化，结合 OLS 回归自适应纠偏。"
                                )
                            }
                            AppDestination.RECORD -> {
                                DestinationPlaceholder(
                                    title = "记录 (食物打卡 & 运动代偿)",
                                    description = "早/中/晚/加餐四餐明细、FTS5 离线食物检索、条形码扫码与 Pontzer 运动代偿结算。"
                                )
                            }
                            AppDestination.BODY -> {
                                DestinationPlaceholder(
                                    title = "身体 (体重历史 & 微量雷达)",
                                    description = "晨起体重与体脂追踪、去脂体重 (LBM) 计算及 NASEM 2023 DRI 微量元素达成度评估。"
                                )
                            }
                            AppDestination.PROFILE -> {
                                DestinationPlaceholder(
                                    title = "个人主页 (档案与目标策略)",
                                    description = "用户生理参数、自适应减重预算、宏量协议切换、多单位制与数据备份导出。"
                                )
                            }
                        }

                        // 纯原生 Compose 液态玻璃悬浮底栏：直接浮动覆盖于滚动内容之上
                        AppNavigationBar(
                            currentRoute = currentDestination.route,
                            onNavigate = { currentDestination = it },
                            modifier = Modifier.align(Alignment.BottomCenter)
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun DestinationPlaceholder(title: String, description: String) {
    Box(
        modifier = Modifier
            .fillMaxSize()
            .background(Color.White)
            .statusBarsPadding()
            .verticalScroll(rememberScrollState()),
        contentAlignment = Alignment.Center
    ) {
        Column(
            horizontalAlignment = Alignment.CenterHorizontally,
            modifier = Modifier.padding(24.dp)
        ) {
            Text(
                text = title,
                style = MaterialTheme.typography.titleLarge,
                color = MaterialTheme.colorScheme.onSurface
            )
            Spacer(modifier = Modifier.height(8.dp))
            Text(
                text = description,
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                textAlign = TextAlign.Center
            )
        }
    }
}
