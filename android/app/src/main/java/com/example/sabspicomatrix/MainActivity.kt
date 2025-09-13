package com.example.sabspicomatrix

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.getValue
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.material3.Button
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.tooling.preview.Preview
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import com.example.sabspicomatrix.ui.theme.SabsPicoMatrixTheme
import com.example.sabspicomatrix.vm.SimpleVM
import dagger.hilt.android.AndroidEntryPoint
import android.Manifest
import android.util.Log
import com.example.sabspicomatrix.repository.BluetoothPowerState
import com.example.sabspicomatrix.repository.Discovery

@AndroidEntryPoint
class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            SabsPicoMatrixTheme {
                Scaffold(
                    modifier = Modifier
                        .fillMaxSize()
                        .safeDrawingPadding()
                ) { innerPadding ->
                    Greeting(
                        name = "Android",
                        modifier = Modifier.padding(innerPadding)
                    )
                }
            }
        }
    }
}

@Composable
fun Greeting(name: String, modifier: Modifier = Modifier, vm: SimpleVM = viewModel()) {
    val launcher =
        rememberLauncherForActivityResult(ActivityResultContracts.RequestMultiplePermissions()) { isGranted ->
            vm.update_permissions()
        }
    val permission by vm.hasBluetoothPermission.collectAsStateWithLifecycle(false)
    val btState by vm.bluetoothPowerState.collectAsStateWithLifecycle()

    val update =
        uniffi.state_bindings.CurrentStateUpdate.GameOfLife(uniffi.state_bindings.GameOfLifeUpdate.RESET)

    Column {
        Text("${uniffi.state_bindings.serializeState(update)}")
        Text(
            "${
                uniffi.state_bindings.serializeState(update)
                    ?.run { uniffi.state_bindings.deserializeState(this) }
            }"
        )
        Text("${uniffi.state_bindings.add(2u, 3u)}")
        Button(onClick = { vm.start_discovery() }) {
            Text("start discovery")
        }
        Text(
            when (btState.powerState) {
                BluetoothPowerState.Unknown -> "unknown"
                BluetoothPowerState.Off -> "off"
                BluetoothPowerState.On -> "on"
                BluetoothPowerState.TurningOff -> "turning off"
                BluetoothPowerState.TurningOn -> "turning on"
            }
        )
        Text(
            when (btState.discovery) {
                Discovery.Finished -> "not discovering"
                Discovery.Started -> "discovering"
                Discovery.Unknown -> "discovery unknown"
            }
        )
        if (permission) {
            Text(
                text = "Hello $name!",
                modifier = modifier
            )
            Text("bluetooth permission: ${permission}")

        } else {
            Button(onClick = {
                Log.d("binding test", "${uniffi.state_bindings.add(2u, 3u)}")


                launcher.launch(
                    arrayOf(
                        Manifest.permission.BLUETOOTH_SCAN,
                        Manifest.permission.BLUETOOTH_CONNECT
                    )
                )
            }) {
                Text("get permission")
            }
        }

    }

}

@Preview(showBackground = true)
@Composable
fun GreetingPreview() {
    SabsPicoMatrixTheme {
        Greeting("Android")
    }
}