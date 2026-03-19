import "dotenv/config";
import { PrismaBetterSqlite3 } from "@prisma/adapter-better-sqlite3";
import { PrismaClient } from "../generated/prisma/client.ts";

const adapter = new PrismaBetterSqlite3({ url: process.env.DATABASE_URL! });
const prisma = new PrismaClient({ adapter });

async function main() {
  // Clean existing data
  await prisma.task.deleteMany();
  await prisma.project.deleteMany();
  await prisma.user.deleteMany();

  // Create users
  const alice = await prisma.user.create({
    data: {
      email: "alice@taskflow.dev",
      name: "Alice Chen",
      role: "admin",
    },
  });

  const bob = await prisma.user.create({
    data: {
      email: "bob@taskflow.dev",
      name: "Bob Martinez",
      role: "member",
    },
  });

  const carol = await prisma.user.create({
    data: {
      email: "carol@taskflow.dev",
      name: "Carol Davis",
      role: "member",
    },
  });

  // Create projects
  const apiProject = await prisma.project.create({
    data: {
      name: "TaskFlow API",
      description: "Core REST API for the TaskFlow platform",
      status: "active",
      ownerId: alice.id,
    },
  });

  const frontendProject = await prisma.project.create({
    data: {
      name: "TaskFlow Frontend",
      description: "React-based web client for TaskFlow",
      status: "active",
      ownerId: bob.id,
    },
  });

  const docsProject = await prisma.project.create({
    data: {
      name: "Documentation",
      description: "API docs and user guides",
      status: "active",
      ownerId: alice.id,
    },
  });

  // Create tasks
  await prisma.task.createMany({
    data: [
      {
        title: "Set up authentication endpoints",
        description: "Implement JWT-based auth with login, register, and refresh",
        status: "in_progress",
        priority: 1,
        projectId: apiProject.id,
        assigneeId: alice.id,
        creatorId: alice.id,
        dueDate: new Date("2026-04-01"),
      },
      {
        title: "Add project CRUD routes",
        description: "REST endpoints for creating, reading, updating, and deleting projects",
        status: "todo",
        priority: 1,
        projectId: apiProject.id,
        assigneeId: bob.id,
        creatorId: alice.id,
        dueDate: new Date("2026-04-05"),
      },
      {
        title: "Implement task assignment",
        description: "Allow assigning tasks to team members with notifications",
        status: "todo",
        priority: 2,
        projectId: apiProject.id,
        assigneeId: carol.id,
        creatorId: alice.id,
        dueDate: new Date("2026-04-10"),
      },
      {
        title: "Design dashboard layout",
        description: "Main dashboard with project overview and task summary",
        status: "done",
        priority: 1,
        projectId: frontendProject.id,
        assigneeId: bob.id,
        creatorId: bob.id,
      },
      {
        title: "Build task board component",
        description: "Kanban-style drag-and-drop task board",
        status: "in_progress",
        priority: 1,
        projectId: frontendProject.id,
        assigneeId: carol.id,
        creatorId: bob.id,
        dueDate: new Date("2026-04-08"),
      },
      {
        title: "Add dark mode support",
        description: "Theme toggle with system preference detection",
        status: "todo",
        priority: 3,
        projectId: frontendProject.id,
        assigneeId: bob.id,
        creatorId: carol.id,
      },
      {
        title: "Write API reference",
        description: "OpenAPI spec and endpoint documentation",
        status: "in_progress",
        priority: 2,
        projectId: docsProject.id,
        assigneeId: alice.id,
        creatorId: alice.id,
        dueDate: new Date("2026-04-15"),
      },
      {
        title: "Create onboarding guide",
        description: "Step-by-step guide for new team members",
        status: "todo",
        priority: 2,
        projectId: docsProject.id,
        assigneeId: carol.id,
        creatorId: alice.id,
      },
    ],
  });

  const userCount = await prisma.user.count();
  const projectCount = await prisma.project.count();
  const taskCount = await prisma.task.count();

  console.log(`Seeded: ${userCount} users, ${projectCount} projects, ${taskCount} tasks`);
}

main()
  .catch((e) => {
    console.error(e);
    process.exit(1);
  })
  .finally(async () => {
    await prisma.$disconnect();
  });
