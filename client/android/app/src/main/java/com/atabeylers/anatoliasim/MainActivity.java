package com.atabeylers.anatoliasim;

import android.os.Bundle;
import com.getcapacitor.BridgeActivity;

public class MainActivity extends BridgeActivity {
    @Override
    public void onCreate(Bundle savedInstanceState) {
        registerPlugin(LocalServerPlugin.class);
        registerPlugin(ApkUpdaterPlugin.class);
        registerPlugin(FileOpenerPlugin.class);
        super.onCreate(savedInstanceState);
    }
}
