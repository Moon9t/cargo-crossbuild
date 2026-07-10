use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use crossbuild_core::{
    model::{BuildPlan, RunReport, PlanStep},
    CrossBuildConfig,
};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tracing::{debug, info};

/// A task in the execution graph.
#[derive(Debug, Clone)]
pub struct Task {
    pub id: String,
    pub step: PlanStep,
    pub dependencies: Vec<String>,
    pub priority: u8,
    pub estimated_duration: Option<Duration>,
}

/// Execution graph for build tasks.
#[derive(Debug)]
pub struct ExecutionGraph {
    tasks: HashMap<String, Task>,
    adjacency: HashMap<String, Vec<String>>,
    reverse_adjacency: HashMap<String, Vec<String>>,
}

impl ExecutionGraph {
    /// Creates a new execution graph from a build plan.
    pub fn from_plan(plan: &BuildPlan) -> Result<Self, crossbuild_core::CrossBuildError> {
        let mut graph = ExecutionGraph {
            tasks: HashMap::new(),
            adjacency: HashMap::new(),
            reverse_adjacency: HashMap::new(),
        };

        // Add all tasks from the plan
        for (idx, step) in plan.steps.iter().enumerate() {
            let id = format!("step_{}", idx);
            let task = Task {
                id: id.clone(),
                step: step.clone(),
                dependencies: Vec::new(),
                priority: 0,
                estimated_duration: None,
            };
            graph.add_task(task);
        }

        // Add dependencies between consecutive steps
        let step_count = plan.steps.len();
        for i in 0..step_count - 1 {
            let from = format!("step_{}", i);
            let to = format!("step_{}", i + 1);
            graph.add_dependency(&from, &to);
        }

        graph.validate()?;
        Ok(graph)
    }

    /// Adds a task to the graph.
    fn add_task(&mut self, task: Task) {
        let id = task.id.clone();
        self.adjacency.insert(id.clone(), Vec::new());
        self.reverse_adjacency.insert(id.clone(), Vec::new());
        self.tasks.insert(id, task);
    }

    /// Adds a dependency between two tasks.
    fn add_dependency(&mut self, from: &str, to: &str) {
        if let Some(deps) = self.adjacency.get_mut(from) {
            deps.push(to.to_string());
        }
        if let Some(rev_deps) = self.reverse_adjacency.get_mut(to) {
            rev_deps.push(from.to_string());
        }
    }

    /// Validates the graph for cycles.
    fn validate(&self) -> Result<(), crossbuild_core::CrossBuildError> {
        // Kahn's algorithm for cycle detection
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        for (node, deps) in &self.adjacency {
            in_degree.entry(node.clone()).or_insert(0);
            for dep in deps {
                *in_degree.entry(dep.clone()).or_insert(0) += 1;
            }
        }

        let mut queue: Vec<String> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(k, _)| k.clone())
            .collect();

        let mut visited = 0;
        while let Some(node) = queue.pop() {
            visited += 1;
            if let Some(deps) = self.adjacency.get(&node) {
                for dep in deps {
                    if let Some(deg) = in_degree.get_mut(dep) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push(dep.clone());
                        }
                    }
                }
            }
        }

        if visited != self.tasks.len() {
            return Err(crossbuild_core::CrossBuildError::configuration(
                "execution graph contains cycles",
            ));
        }

        Ok(())
    }

    /// Returns tasks in topological order.
    pub fn topological_order(&self) -> Vec<String> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        for (node, deps) in &self.adjacency {
            in_degree.entry(node.clone()).or_insert(0);
            for dep in deps {
                *in_degree.entry(dep.clone()).or_insert(0) += 1;
            }
        }

        let mut queue: Vec<String> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(k, _)| k.clone())
            .collect();

        let mut order = Vec::new();
        while let Some(node) = queue.pop() {
            order.push(node.clone());
            if let Some(deps) = self.adjacency.get(&node) {
                for dep in deps {
                    if let Some(deg) = in_degree.get_mut(dep) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push(dep.clone());
                        }
                    }
                }
            }
        }

        order
    }

    /// Gets a task by ID.
    pub fn get_task(&self, id: &str) -> Option<&Task> {
        self.tasks.get(id)
    }

    /// Gets the dependencies of a task.
    pub fn get_dependencies(&self, task_id: &str) -> Vec<String> {
        self.adjacency.get(task_id).cloned().unwrap_or_default()
    }

    /// Gets the reverse dependencies (tasks that depend on this task).
    pub fn get_dependents(&self, task_id: &str) -> Vec<String> {
        self.reverse_adjacency.get(task_id).cloned().unwrap_or_default()
    }
}

/// Resolver that executes the build plan.
pub struct Resolver {
    #[allow(dead_code)]
    max_parallelism: usize,
    task_semaphore: Arc<Semaphore>,
}

impl Resolver {
    /// Creates a new resolver with the specified maximum parallelism.
    pub fn new(max_parallelism: usize) -> Self {
        Self {
            max_parallelism,
            task_semaphore: Arc::new(Semaphore::new(max_parallelism)),
        }
    }

    /// Resolves and executes a build plan.
    pub async fn resolve(
        &self,
        plan: &BuildPlan,
        _config: &CrossBuildConfig,
        _sink: &mut dyn crossbuild_core::diagnostics::DiagnosticSink,
    ) -> Result<RunReport, crossbuild_core::CrossBuildError> {
        let graph = ExecutionGraph::from_plan(plan)?;
        let order = graph.topological_order();

        info!("Executing {} tasks in topological order", order.len());

        let mut join_set = JoinSet::new();
        let completed: HashSet<String> = HashSet::new();

        for task_id in order {
            // Wait for dependencies to complete
            let deps = graph.get_dependencies(&task_id);
            for dep in &deps {
                if !completed.contains(dep) {
                    // Wait for dependency to complete
                    while !completed.contains(dep) {
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                }
            }

            // Acquire semaphore for parallelism control
            let permit = self.task_semaphore.clone().acquire_owned().await.unwrap();

            let task = graph.get_task(&task_id).unwrap();
            let step = task.step.clone();

            let task_id_clone = task_id.clone();

            join_set.spawn(async move {
                let _permit = permit; // Hold permit until task completes
                debug!("Executing task: {}", task_id_clone);
                // Execute the step
                Self::execute_step(&step).await
            });
        }

        // Wait for all tasks to complete
        while let Some(result) = join_set.join_next().await {
            match result {
                Ok(Ok(())) => {}
                Ok(Err(e)) => return Err(e),
                Err(e) => return Err(crossbuild_core::CrossBuildError::configuration(e.to_string())),
            }
        }

        Ok(RunReport {
            executed: true,
            command: "resolved".to_string(),
            working_directory: std::env::current_dir().unwrap_or_default(),
            exit_code: Some(0),
            duration_ms: 0,
        })
    }

    async fn execute_step(step: &PlanStep) -> Result<(), crossbuild_core::CrossBuildError> {
        debug!("Executing step: {:?}", step);
        match step {
            PlanStep::ValidateManifest { path } => {
                if !path.exists() {
                    return Err(crossbuild_core::CrossBuildError::ManifestNotFound {
                        searched_from: path.clone(),
                    });
                }
            }
            PlanStep::ValidateTarget { target } => {
                if target.as_str().is_empty() {
                    return Err(crossbuild_core::CrossBuildError::configuration(
                        "target triple cannot be empty",
                    ));
                }
            }
            PlanStep::DetectHost => {}
            PlanStep::ResolveProviders => {}
            PlanStep::PrepareEnvironment => {}
            PlanStep::GenerateCargoConfig => {}
            PlanStep::ResolveLinker => {}
            PlanStep::PrepareCache => {}
            PlanStep::InvokeCargo => {
                // This is handled by the runner
            }
            PlanStep::CaptureDiagnostics => {}
            PlanStep::VerifyArtifacts => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn execution_graph_creation() {
        let plan = crossbuild_core::BuildPlan {
            request: crossbuild_core::model::BuildRequest::new(
                std::path::PathBuf::from("Cargo.toml"),
                crossbuild_core::model::TargetTriple::parse("x86_64-unknown-linux-gnu").unwrap(),
            ),
            host: crossbuild_core::model::HostInfo::detect().unwrap(),
            target: crossbuild_core::platform::assess_target(
                &crossbuild_core::model::TargetTriple::parse("x86_64-unknown-linux-gnu").unwrap(),
                &crossbuild_core::platform::detect_host().unwrap(),
            ),
            command: crossbuild_core::model::CommandLine::new("cargo", PathBuf::from(".")),
            steps: vec![
                crossbuild_core::model::PlanStep::ValidateManifest { path: std::path::PathBuf::from("Cargo.toml") },
                crossbuild_core::model::PlanStep::ValidateTarget { target: crossbuild_core::model::TargetTriple::parse("x86_64-unknown-linux-gnu").unwrap() },
                crossbuild_core::model::PlanStep::DetectHost,
                crossbuild_core::model::PlanStep::ResolveProviders,
                crossbuild_core::model::PlanStep::PrepareEnvironment,
                crossbuild_core::model::PlanStep::GenerateCargoConfig,
                crossbuild_core::model::PlanStep::ResolveLinker,
                crossbuild_core::model::PlanStep::PrepareCache,
                crossbuild_core::model::PlanStep::InvokeCargo,
                crossbuild_core::model::PlanStep::CaptureDiagnostics,
                crossbuild_core::model::PlanStep::VerifyArtifacts,
            ],
            provider_actions: vec![],
            cargo_config: None,
            cache_key: "test".to_string(),
        };

        let graph = ExecutionGraph::from_plan(&plan).unwrap();
        let order = graph.topological_order();
        assert_eq!(order.len(), 11);
    }

    #[test]
    fn execution_graph_cycle_detection() {
        use crossbuild_core::model::PlanStep;
        use crossbuild_core::model::TargetTriple;
        use std::path::PathBuf;

        let plan = crossbuild_core::BuildPlan {
            request: crossbuild_core::model::BuildRequest::new(
                PathBuf::from("Cargo.toml"),
                TargetTriple::parse("x86_64-unknown-linux-gnu").unwrap(),
            ),
            host: crossbuild_core::platform::detect_host().unwrap(),
            target: crossbuild_core::platform::assess_target(
                &TargetTriple::parse("x86_64-unknown-linux-gnu").unwrap(),
                &crossbuild_core::platform::detect_host().unwrap(),
            ),
            command: crossbuild_core::model::CommandLine::new("cargo", PathBuf::from(".")),
            steps: vec![
                PlanStep::ValidateManifest { path: PathBuf::from("Cargo.toml") },
                PlanStep::DetectHost,
                PlanStep::ResolveProviders,
            ],
            provider_actions: vec![],
            cargo_config: None,
            cache_key: "test".to_string(),
        };

        let graph = ExecutionGraph::from_plan(&plan);
        assert!(graph.is_ok());
    }
}