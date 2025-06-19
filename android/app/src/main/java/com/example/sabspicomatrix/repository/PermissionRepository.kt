package com.example.sabspicomatrix.repository

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import javax.inject.Inject

class PermissionRepository @Inject constructor(private val appContext: Context) {
    private val _btScanPermission: MutableStateFlow<Boolean> =
        MutableStateFlow(this.has_scan_permission())
    private val _btConnectPermission: MutableStateFlow<Boolean> =
        MutableStateFlow(this.has_connect_permission())


    private fun has_scan_permission() =
        appContext.checkSelfPermission(Manifest.permission.BLUETOOTH_SCAN) == PackageManager.PERMISSION_GRANTED

    private fun has_connect_permission() =
        appContext.checkSelfPermission(Manifest.permission.BLUETOOTH_CONNECT) == PackageManager.PERMISSION_GRANTED

    val btScanPermission: StateFlow<Boolean> = _btScanPermission
    val btConnectPermission: StateFlow<Boolean> = _btConnectPermission

    fun update_permissions() {
        _btScanPermission.tryEmit(has_scan_permission())
        _btConnectPermission.tryEmit(has_connect_permission())
    }

}