package com.example.sabspicomatrix.repository

import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothManager
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.util.Log
import androidx.core.content.ContextCompat.getSystemService
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combineTransform
import kotlinx.coroutines.launch
import javax.inject.Inject

sealed class BluetoothPowerState {
    object Unknown : BluetoothPowerState()
    object Off : BluetoothPowerState()
    object TurningOn : BluetoothPowerState()
    object On : BluetoothPowerState()
    object TurningOff : BluetoothPowerState()
    companion object {
        fun from_int(i: Int) =
            when (i) {
                BluetoothAdapter.STATE_OFF -> Off
                BluetoothAdapter.STATE_TURNING_ON -> TurningOn
                BluetoothAdapter.STATE_ON -> On
                BluetoothAdapter.STATE_TURNING_OFF -> TurningOff
                else -> Unknown
            }
    }
}


sealed class Discovery {
    object Finished : Discovery()
    object Started : Discovery()
    object Unknown : Discovery()

    companion object {
        fun from_bool(bool: Boolean) =
            if (bool) {
                Started
            } else {
                Finished
            }
    }
}

data class BluetoothState(val powerState: BluetoothPowerState, val discovery: Discovery)

fun register_intent(
    context: Context,
    onReceiveFun: (Context?, Intent?) -> Unit,
    intent_name: String
) {
    context.registerReceiver(object : BroadcastReceiver() {
        override fun onReceive(p0: Context?, p1: Intent?) {
            when (p1?.action) {
                intent_name -> onReceiveFun(p0, p1)
            }
        }
    }, IntentFilter(intent_name))
}

class BluetoothRepository @Inject constructor(
    private val permissionRepository: PermissionRepository,
    val appContext: Context
) {

    private var manager: BluetoothManager =
        getSystemService(appContext, BluetoothManager::class.java)!!
    private var currentJob: BluetoothJob? = null

    private val _bluetoothState: MutableStateFlow<BluetoothState> =
        MutableStateFlow(
            BluetoothState(
                powerState = BluetoothPowerState.Unknown,
                discovery = Discovery.Unknown
            )
        )

    private fun register_intents() {
        register_intent(appContext, { context, intent ->
            intent?.getIntExtra(BluetoothAdapter.EXTRA_STATE, -1)
                ?.run { emit_updated_bluetooth_power_state(this) }
        }, BluetoothAdapter.ACTION_STATE_CHANGED)
        register_intent(
            appContext,
            { context, intent ->
                emit_updated_bluetooth_discovery_state(true)
            },
            BluetoothAdapter.ACTION_DISCOVERY_STARTED
        )
        register_intent(
            appContext,
            { context, intent ->
                emit_updated_bluetooth_discovery_state(false)
            },
            BluetoothAdapter.ACTION_DISCOVERY_FINISHED
        )
    }


    fun emit_updated_bluetooth_power_state(newState: Int) {
        _bluetoothState.tryEmit(
            _bluetoothState.value.copy(
                powerState = BluetoothPowerState.from_int(
                    newState
                )
            )
        )
    }

    fun emit_updated_bluetooth_discovery_state(newState: Boolean) {
        _bluetoothState.tryEmit(
            _bluetoothState.value.copy(discovery = Discovery.from_bool(newState))
        )
    }

    fun update_bluetooth_state() {
        val powerState: Int = manager.adapter.state
        val discovery: Boolean? = try {
            manager.adapter.isDiscovering
        } catch (e: SecurityException) {
            null
        }
        _bluetoothState.tryEmit(
            BluetoothState(
                powerState = BluetoothPowerState.from_int(powerState),
                discovery = discovery?.let { Discovery.from_bool(it) } ?: Discovery.Unknown))
    }


    init {
        register_intents()
        update_bluetooth_state()
        CoroutineScope(Dispatchers.Default).launch {
            val job = BluetoothJob(this)
            delay(3000)
            job.cancel()
        }

    }

    val hasBluetoothPermission: Flow<Boolean> =
        permissionRepository.btConnectPermission.combineTransform(
            permissionRepository.btScanPermission,
            { p1, p2 ->
                emit(p1 && p2)
            })

    val bluetoothState: StateFlow<BluetoothState> = _bluetoothState

    fun update_permissions() = permissionRepository.update_permissions()

    fun start_discovery() {
        try {
            manager.adapter.startDiscovery()
        } catch (e: SecurityException) {
            Log.d("BluetoothRepository", "No adapter permission")
        }

    }
}