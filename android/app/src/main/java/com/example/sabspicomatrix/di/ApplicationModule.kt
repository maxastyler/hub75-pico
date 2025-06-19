package com.example.sabspicomatrix.di

import android.content.Context
import com.example.sabspicomatrix.repository.BluetoothRepository
import com.example.sabspicomatrix.repository.PermissionRepository
import dagger.Module
import dagger.Provides
import dagger.hilt.InstallIn
import dagger.hilt.android.qualifiers.ApplicationContext
import dagger.hilt.components.SingletonComponent
import javax.inject.Singleton

@Module
@InstallIn(SingletonComponent::class)
class ApplicationModule {
    @Singleton
    @Provides
    fun provideBluetoothRepository(
        permissionRepository: PermissionRepository,
        @ApplicationContext appContext: Context
    ): BluetoothRepository {
        return BluetoothRepository(permissionRepository, appContext)
    }

    @Singleton
    @Provides
    fun providePermissionRepository(@ApplicationContext appContext: Context): PermissionRepository {
        return PermissionRepository(appContext)
    }
}