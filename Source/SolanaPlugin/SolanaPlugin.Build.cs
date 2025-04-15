// Copyright Epic Games, Inc. All Rights Reserved.

using UnrealBuildTool;
using System.IO;

public class SolanaPlugin : ModuleRules
{
    public SolanaPlugin(ReadOnlyTargetRules Target) : base(Target)
    {
        PCHUsage = ModuleRules.PCHUsageMode.UseExplicitOrSharedPCHs;

        // --- Include Paths ---
        PublicIncludePaths.AddRange(
            new string[] {
                Path.Combine(ModuleDirectory, "Public"),
                Path.Combine(ModuleDirectory, "ThirdParty")
            }
        );

        PrivateIncludePaths.AddRange(
            new string[] {
                Path.Combine(ModuleDirectory, "Private")
            }
        );

        // --- Dependencies ---
        PublicDependencyModuleNames.AddRange(
            new string[] { "Core" }
        );

        PrivateDependencyModuleNames.AddRange(
            new string[] {
                "CoreUObject",
                "Engine"
            }
        );

        // --- ThirdParty DLL 설정 ---
        string ThirdPartyPath = Path.Combine(ModuleDirectory, "ThirdParty");

        if (Target.Platform == UnrealTargetPlatform.Win64)
        {
            string PlatformSubfolder = "Win64";
            string LibraryPath = Path.Combine(ThirdPartyPath, PlatformSubfolder);

            string ImportLibName = "solana_c_sdk.dll.lib";
            string DynamicLibName = "solana_c_sdk.dll";

            string ImportLibPath = Path.Combine(LibraryPath, ImportLibName);
            string DynamicLibPath = Path.Combine(LibraryPath, DynamicLibName);

            // .lib 파일
            if (File.Exists(ImportLibPath))
            {
                PublicAdditionalLibraries.Add(ImportLibPath);
                System.Console.WriteLine($"SolanaPlugin: Added import library: {ImportLibPath}");
            }

            // .dll 파일 - ✅ 무조건 ThirdParty에서만 복사하게
            if (File.Exists(DynamicLibPath))
            {
                RuntimeDependencies.Add("$(BinaryOutputDir)/" + DynamicLibName, DynamicLibPath);
                PublicDelayLoadDLLs.Add(DynamicLibName);
                System.Console.WriteLine($"SolanaPlugin: DLL will be copied from: {DynamicLibPath}");
            }
            else
            {
                System.Console.WriteLine($"[ERROR] DLL not found at: {DynamicLibPath}");
            }
        }

    }
}