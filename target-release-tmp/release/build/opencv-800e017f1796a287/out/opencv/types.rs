
mod core_types {
	use crate::{mod_prelude::*, core, types, sys};

	ptr_extern! { core::Algorithm,
		cv_PtrLcv_AlgorithmG_new_null_const, cv_PtrLcv_AlgorithmG_delete, cv_PtrLcv_AlgorithmG_getInnerPtr_const, cv_PtrLcv_AlgorithmG_getInnerPtrMut
	}

	ptr_extern_ctor! { core::Algorithm, cv_PtrLcv_AlgorithmG_new_const_Algorithm }
	impl core::Ptr<core::Algorithm> {
		#[inline] pub fn as_raw_PtrOfAlgorithm(&self) -> extern_send!(Self) { self.as_raw() }
		#[inline] pub fn as_raw_mut_PtrOfAlgorithm(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	impl core::AlgorithmTraitConst for core::Ptr<core::Algorithm> {
		#[inline] fn as_raw_Algorithm(&self) -> *const c_void { self.inner_as_raw() }
	}

	impl core::AlgorithmTrait for core::Ptr<core::Algorithm> {
		#[inline] fn as_raw_mut_Algorithm(&mut self) -> *mut c_void { self.inner_as_raw_mut() }
	}

	impl std::fmt::Debug for core::Ptr<core::Algorithm> {
		#[inline]
		fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
			f.debug_struct("PtrOfAlgorithm")
				.finish()
		}
	}

	ptr_extern! { core::ConjGradSolver,
		cv_PtrLcv_ConjGradSolverG_new_null_const, cv_PtrLcv_ConjGradSolverG_delete, cv_PtrLcv_ConjGradSolverG_getInnerPtr_const, cv_PtrLcv_ConjGradSolverG_getInnerPtrMut
	}

	impl core::Ptr<core::ConjGradSolver> {
		#[inline] pub fn as_raw_PtrOfConjGradSolver(&self) -> extern_send!(Self) { self.as_raw() }
		#[inline] pub fn as_raw_mut_PtrOfConjGradSolver(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	impl core::ConjGradSolverTraitConst for core::Ptr<core::ConjGradSolver> {
		#[inline] fn as_raw_ConjGradSolver(&self) -> *const c_void { self.inner_as_raw() }
	}

	impl core::ConjGradSolverTrait for core::Ptr<core::ConjGradSolver> {
		#[inline] fn as_raw_mut_ConjGradSolver(&mut self) -> *mut c_void { self.inner_as_raw_mut() }
	}

	impl core::AlgorithmTraitConst for core::Ptr<core::ConjGradSolver> {
		#[inline] fn as_raw_Algorithm(&self) -> *const c_void { self.inner_as_raw() }
	}

	impl core::AlgorithmTrait for core::Ptr<core::ConjGradSolver> {
		#[inline] fn as_raw_mut_Algorithm(&mut self) -> *mut c_void { self.inner_as_raw_mut() }
	}

	ptr_cast_base! { core::Ptr<core::ConjGradSolver>, core::Ptr<core::Algorithm>, cv_PtrLcv_ConjGradSolverG_to_PtrOfAlgorithm }

	impl core::MinProblemSolverTraitConst for core::Ptr<core::ConjGradSolver> {
		#[inline] fn as_raw_MinProblemSolver(&self) -> *const c_void { self.inner_as_raw() }
	}

	impl core::MinProblemSolverTrait for core::Ptr<core::ConjGradSolver> {
		#[inline] fn as_raw_mut_MinProblemSolver(&mut self) -> *mut c_void { self.inner_as_raw_mut() }
	}

	ptr_cast_base! { core::Ptr<core::ConjGradSolver>, core::Ptr<core::MinProblemSolver>, cv_PtrLcv_ConjGradSolverG_to_PtrOfMinProblemSolver }

	impl std::fmt::Debug for core::Ptr<core::ConjGradSolver> {
		#[inline]
		fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
			f.debug_struct("PtrOfConjGradSolver")
				.finish()
		}
	}

	ptr_extern! { core::DownhillSolver,
		cv_PtrLcv_DownhillSolverG_new_null_const, cv_PtrLcv_DownhillSolverG_delete, cv_PtrLcv_DownhillSolverG_getInnerPtr_const, cv_PtrLcv_DownhillSolverG_getInnerPtrMut
	}

	impl core::Ptr<core::DownhillSolver> {
		#[inline] pub fn as_raw_PtrOfDownhillSolver(&self) -> extern_send!(Self) { self.as_raw() }
		#[inline] pub fn as_raw_mut_PtrOfDownhillSolver(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	impl core::DownhillSolverTraitConst for core::Ptr<core::DownhillSolver> {
		#[inline] fn as_raw_DownhillSolver(&self) -> *const c_void { self.inner_as_raw() }
	}

	impl core::DownhillSolverTrait for core::Ptr<core::DownhillSolver> {
		#[inline] fn as_raw_mut_DownhillSolver(&mut self) -> *mut c_void { self.inner_as_raw_mut() }
	}

	impl core::AlgorithmTraitConst for core::Ptr<core::DownhillSolver> {
		#[inline] fn as_raw_Algorithm(&self) -> *const c_void { self.inner_as_raw() }
	}

	impl core::AlgorithmTrait for core::Ptr<core::DownhillSolver> {
		#[inline] fn as_raw_mut_Algorithm(&mut self) -> *mut c_void { self.inner_as_raw_mut() }
	}

	ptr_cast_base! { core::Ptr<core::DownhillSolver>, core::Ptr<core::Algorithm>, cv_PtrLcv_DownhillSolverG_to_PtrOfAlgorithm }

	impl core::MinProblemSolverTraitConst for core::Ptr<core::DownhillSolver> {
		#[inline] fn as_raw_MinProblemSolver(&self) -> *const c_void { self.inner_as_raw() }
	}

	impl core::MinProblemSolverTrait for core::Ptr<core::DownhillSolver> {
		#[inline] fn as_raw_mut_MinProblemSolver(&mut self) -> *mut c_void { self.inner_as_raw_mut() }
	}

	ptr_cast_base! { core::Ptr<core::DownhillSolver>, core::Ptr<core::MinProblemSolver>, cv_PtrLcv_DownhillSolverG_to_PtrOfMinProblemSolver }

	impl std::fmt::Debug for core::Ptr<core::DownhillSolver> {
		#[inline]
		fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
			f.debug_struct("PtrOfDownhillSolver")
				.finish()
		}
	}

	ptr_extern! { core::FileStorage,
		cv_PtrLcv_FileStorageG_new_null_const, cv_PtrLcv_FileStorageG_delete, cv_PtrLcv_FileStorageG_getInnerPtr_const, cv_PtrLcv_FileStorageG_getInnerPtrMut
	}

	ptr_extern_ctor! { core::FileStorage, cv_PtrLcv_FileStorageG_new_const_FileStorage }
	impl core::Ptr<core::FileStorage> {
		#[inline] pub fn as_raw_PtrOfFileStorage(&self) -> extern_send!(Self) { self.as_raw() }
		#[inline] pub fn as_raw_mut_PtrOfFileStorage(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	impl core::FileStorageTraitConst for core::Ptr<core::FileStorage> {
		#[inline] fn as_raw_FileStorage(&self) -> *const c_void { self.inner_as_raw() }
	}

	impl core::FileStorageTrait for core::Ptr<core::FileStorage> {
		#[inline] fn as_raw_mut_FileStorage(&mut self) -> *mut c_void { self.inner_as_raw_mut() }
	}

	impl std::fmt::Debug for core::Ptr<core::FileStorage> {
		#[inline]
		fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
			f.debug_struct("PtrOfFileStorage")
				.field("state", &core::FileStorageTraitConst::state(self))
				.field("elname", &core::FileStorageTraitConst::elname(self))
				.finish()
		}
	}

	ptr_extern! { core::Formatted,
		cv_PtrLcv_FormattedG_new_null_const, cv_PtrLcv_FormattedG_delete, cv_PtrLcv_FormattedG_getInnerPtr_const, cv_PtrLcv_FormattedG_getInnerPtrMut
	}

	impl core::Ptr<core::Formatted> {
		#[inline] pub fn as_raw_PtrOfFormatted(&self) -> extern_send!(Self) { self.as_raw() }
		#[inline] pub fn as_raw_mut_PtrOfFormatted(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	impl core::FormattedTraitConst for core::Ptr<core::Formatted> {
		#[inline] fn as_raw_Formatted(&self) -> *const c_void { self.inner_as_raw() }
	}

	impl core::FormattedTrait for core::Ptr<core::Formatted> {
		#[inline] fn as_raw_mut_Formatted(&mut self) -> *mut c_void { self.inner_as_raw_mut() }
	}

	impl std::fmt::Debug for core::Ptr<core::Formatted> {
		#[inline]
		fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
			f.debug_struct("PtrOfFormatted")
				.finish()
		}
	}

	ptr_extern! { core::Formatter,
		cv_PtrLcv_FormatterG_new_null_const, cv_PtrLcv_FormatterG_delete, cv_PtrLcv_FormatterG_getInnerPtr_const, cv_PtrLcv_FormatterG_getInnerPtrMut
	}

	impl core::Ptr<core::Formatter> {
		#[inline] pub fn as_raw_PtrOfFormatter(&self) -> extern_send!(Self) { self.as_raw() }
		#[inline] pub fn as_raw_mut_PtrOfFormatter(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	impl core::FormatterTraitConst for core::Ptr<core::Formatter> {
		#[inline] fn as_raw_Formatter(&self) -> *const c_void { self.inner_as_raw() }
	}

	impl core::FormatterTrait for core::Ptr<core::Formatter> {
		#[inline] fn as_raw_mut_Formatter(&mut self) -> *mut c_void { self.inner_as_raw_mut() }
	}

	impl std::fmt::Debug for core::Ptr<core::Formatter> {
		#[inline]
		fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
			f.debug_struct("PtrOfFormatter")
				.finish()
		}
	}

	ptr_extern! { core::KeyPoint,
		cv_PtrLcv_KeyPointG_new_null_const, cv_PtrLcv_KeyPointG_delete, cv_PtrLcv_KeyPointG_getInnerPtr_const, cv_PtrLcv_KeyPointG_getInnerPtrMut
	}

	ptr_extern_ctor! { core::KeyPoint, cv_PtrLcv_KeyPointG_new_const_KeyPoint }
	impl core::Ptr<core::KeyPoint> {
		#[inline] pub fn as_raw_PtrOfKeyPoint(&self) -> extern_send!(Self) { self.as_raw() }
		#[inline] pub fn as_raw_mut_PtrOfKeyPoint(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	impl core::KeyPointTraitConst for core::Ptr<core::KeyPoint> {
		#[inline] fn as_raw_KeyPoint(&self) -> *const c_void { self.inner_as_raw() }
	}

	impl core::KeyPointTrait for core::Ptr<core::KeyPoint> {
		#[inline] fn as_raw_mut_KeyPoint(&mut self) -> *mut c_void { self.inner_as_raw_mut() }
	}

	impl std::fmt::Debug for core::Ptr<core::KeyPoint> {
		#[inline]
		fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
			f.debug_struct("PtrOfKeyPoint")
				.finish()
		}
	}

	ptr_extern! { core::MinProblemSolver,
		cv_PtrLcv_MinProblemSolverG_new_null_const, cv_PtrLcv_MinProblemSolverG_delete, cv_PtrLcv_MinProblemSolverG_getInnerPtr_const, cv_PtrLcv_MinProblemSolverG_getInnerPtrMut
	}

	impl core::Ptr<core::MinProblemSolver> {
		#[inline] pub fn as_raw_PtrOfMinProblemSolver(&self) -> extern_send!(Self) { self.as_raw() }
		#[inline] pub fn as_raw_mut_PtrOfMinProblemSolver(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	impl core::MinProblemSolverTraitConst for core::Ptr<core::MinProblemSolver> {
		#[inline] fn as_raw_MinProblemSolver(&self) -> *const c_void { self.inner_as_raw() }
	}

	impl core::MinProblemSolverTrait for core::Ptr<core::MinProblemSolver> {
		#[inline] fn as_raw_mut_MinProblemSolver(&mut self) -> *mut c_void { self.inner_as_raw_mut() }
	}

	impl core::AlgorithmTraitConst for core::Ptr<core::MinProblemSolver> {
		#[inline] fn as_raw_Algorithm(&self) -> *const c_void { self.inner_as_raw() }
	}

	impl core::AlgorithmTrait for core::Ptr<core::MinProblemSolver> {
		#[inline] fn as_raw_mut_Algorithm(&mut self) -> *mut c_void { self.inner_as_raw_mut() }
	}

	ptr_cast_base! { core::Ptr<core::MinProblemSolver>, core::Ptr<core::Algorithm>, cv_PtrLcv_MinProblemSolverG_to_PtrOfAlgorithm }

	impl std::fmt::Debug for core::Ptr<core::MinProblemSolver> {
		#[inline]
		fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
			f.debug_struct("PtrOfMinProblemSolver")
				.finish()
		}
	}

	ptr_extern! { core::MinProblemSolver_Function,
		cv_PtrLcv_MinProblemSolver_FunctionG_new_null_const, cv_PtrLcv_MinProblemSolver_FunctionG_delete, cv_PtrLcv_MinProblemSolver_FunctionG_getInnerPtr_const, cv_PtrLcv_MinProblemSolver_FunctionG_getInnerPtrMut
	}

	impl core::Ptr<core::MinProblemSolver_Function> {
		#[inline] pub fn as_raw_PtrOfMinProblemSolver_Function(&self) -> extern_send!(Self) { self.as_raw() }
		#[inline] pub fn as_raw_mut_PtrOfMinProblemSolver_Function(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	impl core::MinProblemSolver_FunctionTraitConst for core::Ptr<core::MinProblemSolver_Function> {
		#[inline] fn as_raw_MinProblemSolver_Function(&self) -> *const c_void { self.inner_as_raw() }
	}

	impl core::MinProblemSolver_FunctionTrait for core::Ptr<core::MinProblemSolver_Function> {
		#[inline] fn as_raw_mut_MinProblemSolver_Function(&mut self) -> *mut c_void { self.inner_as_raw_mut() }
	}

	impl std::fmt::Debug for core::Ptr<core::MinProblemSolver_Function> {
		#[inline]
		fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
			f.debug_struct("PtrOfMinProblemSolver_Function")
				.finish()
		}
	}

	ptr_extern! { core::OriginalClassName,
		cv_PtrLcv_utils_nested_OriginalClassNameG_new_null_const, cv_PtrLcv_utils_nested_OriginalClassNameG_delete, cv_PtrLcv_utils_nested_OriginalClassNameG_getInnerPtr_const, cv_PtrLcv_utils_nested_OriginalClassNameG_getInnerPtrMut
	}

	ptr_extern_ctor! { core::OriginalClassName, cv_PtrLcv_utils_nested_OriginalClassNameG_new_const_OriginalClassName }
	impl core::Ptr<core::OriginalClassName> {
		#[inline] pub fn as_raw_PtrOfOriginalClassName(&self) -> extern_send!(Self) { self.as_raw() }
		#[inline] pub fn as_raw_mut_PtrOfOriginalClassName(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	impl core::OriginalClassNameTraitConst for core::Ptr<core::OriginalClassName> {
		#[inline] fn as_raw_OriginalClassName(&self) -> *const c_void { self.inner_as_raw() }
	}

	impl core::OriginalClassNameTrait for core::Ptr<core::OriginalClassName> {
		#[inline] fn as_raw_mut_OriginalClassName(&mut self) -> *mut c_void { self.inner_as_raw_mut() }
	}

	impl std::fmt::Debug for core::Ptr<core::OriginalClassName> {
		#[inline]
		fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
			f.debug_struct("PtrOfOriginalClassName")
				.finish()
		}
	}

	ptr_extern! { f32,
		cv_PtrLfloatG_new_null_const, cv_PtrLfloatG_delete, cv_PtrLfloatG_getInnerPtr_const, cv_PtrLfloatG_getInnerPtrMut
	}

	ptr_extern_ctor! { f32, cv_PtrLfloatG_new_const_float }
	impl core::Ptr<f32> {
		#[inline] pub fn as_raw_PtrOff32(&self) -> extern_send!(Self) { self.as_raw() }
		#[inline] pub fn as_raw_mut_PtrOff32(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	impl core::Vector<core::DMatch> {
		pub fn as_raw_VectorOfDMatch(&self) -> extern_send!(Self) { self.as_raw() }
		pub fn as_raw_mut_VectorOfDMatch(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	vector_extern! { core::DMatch,
		std_vectorLcv_DMatchG_new_const, std_vectorLcv_DMatchG_delete,
		std_vectorLcv_DMatchG_len_const, std_vectorLcv_DMatchG_isEmpty_const,
		std_vectorLcv_DMatchG_capacity_const, std_vectorLcv_DMatchG_shrinkToFit,
		std_vectorLcv_DMatchG_reserve_size_t, std_vectorLcv_DMatchG_remove_size_t,
		std_vectorLcv_DMatchG_swap_size_t_size_t, std_vectorLcv_DMatchG_clear,
		std_vectorLcv_DMatchG_get_const_size_t, std_vectorLcv_DMatchG_set_size_t_const_DMatch,
		std_vectorLcv_DMatchG_push_const_DMatch, std_vectorLcv_DMatchG_insert_size_t_const_DMatch,
	}

	vector_copy_non_bool! { core::DMatch,
		std_vectorLcv_DMatchG_data_const, std_vectorLcv_DMatchG_dataMut, cv_fromSlice_const_const_DMatchX_size_t,
		std_vectorLcv_DMatchG_clone_const,
	}


	impl core::Vector<core::KeyPoint> {
		pub fn as_raw_VectorOfKeyPoint(&self) -> extern_send!(Self) { self.as_raw() }
		pub fn as_raw_mut_VectorOfKeyPoint(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	vector_extern! { core::KeyPoint,
		std_vectorLcv_KeyPointG_new_const, std_vectorLcv_KeyPointG_delete,
		std_vectorLcv_KeyPointG_len_const, std_vectorLcv_KeyPointG_isEmpty_const,
		std_vectorLcv_KeyPointG_capacity_const, std_vectorLcv_KeyPointG_shrinkToFit,
		std_vectorLcv_KeyPointG_reserve_size_t, std_vectorLcv_KeyPointG_remove_size_t,
		std_vectorLcv_KeyPointG_swap_size_t_size_t, std_vectorLcv_KeyPointG_clear,
		std_vectorLcv_KeyPointG_get_const_size_t, std_vectorLcv_KeyPointG_set_size_t_const_KeyPoint,
		std_vectorLcv_KeyPointG_push_const_KeyPoint, std_vectorLcv_KeyPointG_insert_size_t_const_KeyPoint,
	}

	vector_non_copy_or_bool! { clone core::KeyPoint }

	vector_boxed_ref! { core::KeyPoint }

	vector_extern! { BoxedRef<'t, core::KeyPoint>,
		std_vectorLcv_KeyPointG_new_const, std_vectorLcv_KeyPointG_delete,
		std_vectorLcv_KeyPointG_len_const, std_vectorLcv_KeyPointG_isEmpty_const,
		std_vectorLcv_KeyPointG_capacity_const, std_vectorLcv_KeyPointG_shrinkToFit,
		std_vectorLcv_KeyPointG_reserve_size_t, std_vectorLcv_KeyPointG_remove_size_t,
		std_vectorLcv_KeyPointG_swap_size_t_size_t, std_vectorLcv_KeyPointG_clear,
		std_vectorLcv_KeyPointG_get_const_size_t, std_vectorLcv_KeyPointG_set_size_t_const_KeyPoint,
		std_vectorLcv_KeyPointG_push_const_KeyPoint, std_vectorLcv_KeyPointG_insert_size_t_const_KeyPoint,
	}


	impl core::Vector<core::Mat> {
		pub fn as_raw_VectorOfMat(&self) -> extern_send!(Self) { self.as_raw() }
		pub fn as_raw_mut_VectorOfMat(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	vector_extern! { core::Mat,
		std_vectorLcv_MatG_new_const, std_vectorLcv_MatG_delete,
		std_vectorLcv_MatG_len_const, std_vectorLcv_MatG_isEmpty_const,
		std_vectorLcv_MatG_capacity_const, std_vectorLcv_MatG_shrinkToFit,
		std_vectorLcv_MatG_reserve_size_t, std_vectorLcv_MatG_remove_size_t,
		std_vectorLcv_MatG_swap_size_t_size_t, std_vectorLcv_MatG_clear,
		std_vectorLcv_MatG_get_const_size_t, std_vectorLcv_MatG_set_size_t_const_Mat,
		std_vectorLcv_MatG_push_const_Mat, std_vectorLcv_MatG_insert_size_t_const_Mat,
	}

	vector_non_copy_or_bool! { clone core::Mat }

	vector_boxed_ref! { core::Mat }

	vector_extern! { BoxedRef<'t, core::Mat>,
		std_vectorLcv_MatG_new_const, std_vectorLcv_MatG_delete,
		std_vectorLcv_MatG_len_const, std_vectorLcv_MatG_isEmpty_const,
		std_vectorLcv_MatG_capacity_const, std_vectorLcv_MatG_shrinkToFit,
		std_vectorLcv_MatG_reserve_size_t, std_vectorLcv_MatG_remove_size_t,
		std_vectorLcv_MatG_swap_size_t_size_t, std_vectorLcv_MatG_clear,
		std_vectorLcv_MatG_get_const_size_t, std_vectorLcv_MatG_set_size_t_const_Mat,
		std_vectorLcv_MatG_push_const_Mat, std_vectorLcv_MatG_insert_size_t_const_Mat,
	}


	impl core::Vector<core::PlatformInfo> {
		pub fn as_raw_VectorOfPlatformInfo(&self) -> extern_send!(Self) { self.as_raw() }
		pub fn as_raw_mut_VectorOfPlatformInfo(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	vector_extern! { core::PlatformInfo,
		std_vectorLcv_ocl_PlatformInfoG_new_const, std_vectorLcv_ocl_PlatformInfoG_delete,
		std_vectorLcv_ocl_PlatformInfoG_len_const, std_vectorLcv_ocl_PlatformInfoG_isEmpty_const,
		std_vectorLcv_ocl_PlatformInfoG_capacity_const, std_vectorLcv_ocl_PlatformInfoG_shrinkToFit,
		std_vectorLcv_ocl_PlatformInfoG_reserve_size_t, std_vectorLcv_ocl_PlatformInfoG_remove_size_t,
		std_vectorLcv_ocl_PlatformInfoG_swap_size_t_size_t, std_vectorLcv_ocl_PlatformInfoG_clear,
		std_vectorLcv_ocl_PlatformInfoG_get_const_size_t, std_vectorLcv_ocl_PlatformInfoG_set_size_t_const_PlatformInfo,
		std_vectorLcv_ocl_PlatformInfoG_push_const_PlatformInfo, std_vectorLcv_ocl_PlatformInfoG_insert_size_t_const_PlatformInfo,
	}

	vector_non_copy_or_bool! { core::PlatformInfo }

	vector_boxed_ref! { core::PlatformInfo }

	vector_extern! { BoxedRef<'t, core::PlatformInfo>,
		std_vectorLcv_ocl_PlatformInfoG_new_const, std_vectorLcv_ocl_PlatformInfoG_delete,
		std_vectorLcv_ocl_PlatformInfoG_len_const, std_vectorLcv_ocl_PlatformInfoG_isEmpty_const,
		std_vectorLcv_ocl_PlatformInfoG_capacity_const, std_vectorLcv_ocl_PlatformInfoG_shrinkToFit,
		std_vectorLcv_ocl_PlatformInfoG_reserve_size_t, std_vectorLcv_ocl_PlatformInfoG_remove_size_t,
		std_vectorLcv_ocl_PlatformInfoG_swap_size_t_size_t, std_vectorLcv_ocl_PlatformInfoG_clear,
		std_vectorLcv_ocl_PlatformInfoG_get_const_size_t, std_vectorLcv_ocl_PlatformInfoG_set_size_t_const_PlatformInfo,
		std_vectorLcv_ocl_PlatformInfoG_push_const_PlatformInfo, std_vectorLcv_ocl_PlatformInfoG_insert_size_t_const_PlatformInfo,
	}


	impl core::Vector<core::Point> {
		pub fn as_raw_VectorOfPoint(&self) -> extern_send!(Self) { self.as_raw() }
		pub fn as_raw_mut_VectorOfPoint(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	vector_extern! { core::Point,
		std_vectorLcv_PointG_new_const, std_vectorLcv_PointG_delete,
		std_vectorLcv_PointG_len_const, std_vectorLcv_PointG_isEmpty_const,
		std_vectorLcv_PointG_capacity_const, std_vectorLcv_PointG_shrinkToFit,
		std_vectorLcv_PointG_reserve_size_t, std_vectorLcv_PointG_remove_size_t,
		std_vectorLcv_PointG_swap_size_t_size_t, std_vectorLcv_PointG_clear,
		std_vectorLcv_PointG_get_const_size_t, std_vectorLcv_PointG_set_size_t_const_Point,
		std_vectorLcv_PointG_push_const_Point, std_vectorLcv_PointG_insert_size_t_const_Point,
	}

	vector_copy_non_bool! { core::Point,
		std_vectorLcv_PointG_data_const, std_vectorLcv_PointG_dataMut, cv_fromSlice_const_const_PointX_size_t,
		std_vectorLcv_PointG_clone_const,
	}

	impl ToInputArray for core::Vector<core::Point> {
		#[inline]
		fn input_array(&self) -> Result<BoxedRef<'_, core::_InputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLcv_PointG_inputArray_const(self.as_raw_VectorOfPoint(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRef::<'_, core::_InputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	input_array_ref_forward! { core::Vector<core::Point> }

	impl ToOutputArray for core::Vector<core::Point> {
		#[inline]
		fn output_array(&mut self) -> Result<BoxedRefMut<'_, core::_OutputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLcv_PointG_outputArray(self.as_raw_mut_VectorOfPoint(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRefMut::<'_, core::_OutputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	impl ToInputOutputArray for core::Vector<core::Point> {
		#[inline]
		fn input_output_array(&mut self) -> Result<BoxedRefMut<'_, core::_InputOutputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLcv_PointG_inputOutputArray(self.as_raw_mut_VectorOfPoint(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRefMut::<'_, core::_InputOutputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	output_array_ref_forward! { core::Vector<core::Point> }

	impl core::Vector<core::Point2d> {
		pub fn as_raw_VectorOfPoint2d(&self) -> extern_send!(Self) { self.as_raw() }
		pub fn as_raw_mut_VectorOfPoint2d(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	vector_extern! { core::Point2d,
		std_vectorLcv_Point2dG_new_const, std_vectorLcv_Point2dG_delete,
		std_vectorLcv_Point2dG_len_const, std_vectorLcv_Point2dG_isEmpty_const,
		std_vectorLcv_Point2dG_capacity_const, std_vectorLcv_Point2dG_shrinkToFit,
		std_vectorLcv_Point2dG_reserve_size_t, std_vectorLcv_Point2dG_remove_size_t,
		std_vectorLcv_Point2dG_swap_size_t_size_t, std_vectorLcv_Point2dG_clear,
		std_vectorLcv_Point2dG_get_const_size_t, std_vectorLcv_Point2dG_set_size_t_const_Point2d,
		std_vectorLcv_Point2dG_push_const_Point2d, std_vectorLcv_Point2dG_insert_size_t_const_Point2d,
	}

	vector_copy_non_bool! { core::Point2d,
		std_vectorLcv_Point2dG_data_const, std_vectorLcv_Point2dG_dataMut, cv_fromSlice_const_const_Point2dX_size_t,
		std_vectorLcv_Point2dG_clone_const,
	}

	impl ToInputArray for core::Vector<core::Point2d> {
		#[inline]
		fn input_array(&self) -> Result<BoxedRef<'_, core::_InputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLcv_Point2dG_inputArray_const(self.as_raw_VectorOfPoint2d(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRef::<'_, core::_InputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	input_array_ref_forward! { core::Vector<core::Point2d> }

	impl ToOutputArray for core::Vector<core::Point2d> {
		#[inline]
		fn output_array(&mut self) -> Result<BoxedRefMut<'_, core::_OutputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLcv_Point2dG_outputArray(self.as_raw_mut_VectorOfPoint2d(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRefMut::<'_, core::_OutputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	impl ToInputOutputArray for core::Vector<core::Point2d> {
		#[inline]
		fn input_output_array(&mut self) -> Result<BoxedRefMut<'_, core::_InputOutputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLcv_Point2dG_inputOutputArray(self.as_raw_mut_VectorOfPoint2d(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRefMut::<'_, core::_InputOutputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	output_array_ref_forward! { core::Vector<core::Point2d> }

	impl core::Vector<core::Point2f> {
		pub fn as_raw_VectorOfPoint2f(&self) -> extern_send!(Self) { self.as_raw() }
		pub fn as_raw_mut_VectorOfPoint2f(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	vector_extern! { core::Point2f,
		std_vectorLcv_Point2fG_new_const, std_vectorLcv_Point2fG_delete,
		std_vectorLcv_Point2fG_len_const, std_vectorLcv_Point2fG_isEmpty_const,
		std_vectorLcv_Point2fG_capacity_const, std_vectorLcv_Point2fG_shrinkToFit,
		std_vectorLcv_Point2fG_reserve_size_t, std_vectorLcv_Point2fG_remove_size_t,
		std_vectorLcv_Point2fG_swap_size_t_size_t, std_vectorLcv_Point2fG_clear,
		std_vectorLcv_Point2fG_get_const_size_t, std_vectorLcv_Point2fG_set_size_t_const_Point2f,
		std_vectorLcv_Point2fG_push_const_Point2f, std_vectorLcv_Point2fG_insert_size_t_const_Point2f,
	}

	vector_copy_non_bool! { core::Point2f,
		std_vectorLcv_Point2fG_data_const, std_vectorLcv_Point2fG_dataMut, cv_fromSlice_const_const_Point2fX_size_t,
		std_vectorLcv_Point2fG_clone_const,
	}

	impl ToInputArray for core::Vector<core::Point2f> {
		#[inline]
		fn input_array(&self) -> Result<BoxedRef<'_, core::_InputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLcv_Point2fG_inputArray_const(self.as_raw_VectorOfPoint2f(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRef::<'_, core::_InputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	input_array_ref_forward! { core::Vector<core::Point2f> }

	impl ToOutputArray for core::Vector<core::Point2f> {
		#[inline]
		fn output_array(&mut self) -> Result<BoxedRefMut<'_, core::_OutputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLcv_Point2fG_outputArray(self.as_raw_mut_VectorOfPoint2f(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRefMut::<'_, core::_OutputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	impl ToInputOutputArray for core::Vector<core::Point2f> {
		#[inline]
		fn input_output_array(&mut self) -> Result<BoxedRefMut<'_, core::_InputOutputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLcv_Point2fG_inputOutputArray(self.as_raw_mut_VectorOfPoint2f(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRefMut::<'_, core::_InputOutputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	output_array_ref_forward! { core::Vector<core::Point2f> }

	impl core::Vector<core::Range> {
		pub fn as_raw_VectorOfRange(&self) -> extern_send!(Self) { self.as_raw() }
		pub fn as_raw_mut_VectorOfRange(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	vector_extern! { core::Range,
		std_vectorLcv_RangeG_new_const, std_vectorLcv_RangeG_delete,
		std_vectorLcv_RangeG_len_const, std_vectorLcv_RangeG_isEmpty_const,
		std_vectorLcv_RangeG_capacity_const, std_vectorLcv_RangeG_shrinkToFit,
		std_vectorLcv_RangeG_reserve_size_t, std_vectorLcv_RangeG_remove_size_t,
		std_vectorLcv_RangeG_swap_size_t_size_t, std_vectorLcv_RangeG_clear,
		std_vectorLcv_RangeG_get_const_size_t, std_vectorLcv_RangeG_set_size_t_const_Range,
		std_vectorLcv_RangeG_push_const_Range, std_vectorLcv_RangeG_insert_size_t_const_Range,
	}

	vector_non_copy_or_bool! { core::Range }

	vector_boxed_ref! { core::Range }

	vector_extern! { BoxedRef<'t, core::Range>,
		std_vectorLcv_RangeG_new_const, std_vectorLcv_RangeG_delete,
		std_vectorLcv_RangeG_len_const, std_vectorLcv_RangeG_isEmpty_const,
		std_vectorLcv_RangeG_capacity_const, std_vectorLcv_RangeG_shrinkToFit,
		std_vectorLcv_RangeG_reserve_size_t, std_vectorLcv_RangeG_remove_size_t,
		std_vectorLcv_RangeG_swap_size_t_size_t, std_vectorLcv_RangeG_clear,
		std_vectorLcv_RangeG_get_const_size_t, std_vectorLcv_RangeG_set_size_t_const_Range,
		std_vectorLcv_RangeG_push_const_Range, std_vectorLcv_RangeG_insert_size_t_const_Range,
	}


	impl core::Vector<core::Rect> {
		pub fn as_raw_VectorOfRect(&self) -> extern_send!(Self) { self.as_raw() }
		pub fn as_raw_mut_VectorOfRect(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	vector_extern! { core::Rect,
		std_vectorLcv_RectG_new_const, std_vectorLcv_RectG_delete,
		std_vectorLcv_RectG_len_const, std_vectorLcv_RectG_isEmpty_const,
		std_vectorLcv_RectG_capacity_const, std_vectorLcv_RectG_shrinkToFit,
		std_vectorLcv_RectG_reserve_size_t, std_vectorLcv_RectG_remove_size_t,
		std_vectorLcv_RectG_swap_size_t_size_t, std_vectorLcv_RectG_clear,
		std_vectorLcv_RectG_get_const_size_t, std_vectorLcv_RectG_set_size_t_const_Rect,
		std_vectorLcv_RectG_push_const_Rect, std_vectorLcv_RectG_insert_size_t_const_Rect,
	}

	vector_copy_non_bool! { core::Rect,
		std_vectorLcv_RectG_data_const, std_vectorLcv_RectG_dataMut, cv_fromSlice_const_const_RectX_size_t,
		std_vectorLcv_RectG_clone_const,
	}

	impl ToInputArray for core::Vector<core::Rect> {
		#[inline]
		fn input_array(&self) -> Result<BoxedRef<'_, core::_InputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLcv_RectG_inputArray_const(self.as_raw_VectorOfRect(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRef::<'_, core::_InputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	input_array_ref_forward! { core::Vector<core::Rect> }

	impl ToOutputArray for core::Vector<core::Rect> {
		#[inline]
		fn output_array(&mut self) -> Result<BoxedRefMut<'_, core::_OutputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLcv_RectG_outputArray(self.as_raw_mut_VectorOfRect(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRefMut::<'_, core::_OutputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	impl ToInputOutputArray for core::Vector<core::Rect> {
		#[inline]
		fn input_output_array(&mut self) -> Result<BoxedRefMut<'_, core::_InputOutputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLcv_RectG_inputOutputArray(self.as_raw_mut_VectorOfRect(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRefMut::<'_, core::_InputOutputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	output_array_ref_forward! { core::Vector<core::Rect> }

	impl core::Vector<core::RotatedRect> {
		pub fn as_raw_VectorOfRotatedRect(&self) -> extern_send!(Self) { self.as_raw() }
		pub fn as_raw_mut_VectorOfRotatedRect(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	vector_extern! { core::RotatedRect,
		std_vectorLcv_RotatedRectG_new_const, std_vectorLcv_RotatedRectG_delete,
		std_vectorLcv_RotatedRectG_len_const, std_vectorLcv_RotatedRectG_isEmpty_const,
		std_vectorLcv_RotatedRectG_capacity_const, std_vectorLcv_RotatedRectG_shrinkToFit,
		std_vectorLcv_RotatedRectG_reserve_size_t, std_vectorLcv_RotatedRectG_remove_size_t,
		std_vectorLcv_RotatedRectG_swap_size_t_size_t, std_vectorLcv_RotatedRectG_clear,
		std_vectorLcv_RotatedRectG_get_const_size_t, std_vectorLcv_RotatedRectG_set_size_t_const_RotatedRect,
		std_vectorLcv_RotatedRectG_push_const_RotatedRect, std_vectorLcv_RotatedRectG_insert_size_t_const_RotatedRect,
	}

	vector_copy_non_bool! { core::RotatedRect,
		std_vectorLcv_RotatedRectG_data_const, std_vectorLcv_RotatedRectG_dataMut, cv_fromSlice_const_const_RotatedRectX_size_t,
		std_vectorLcv_RotatedRectG_clone_const,
	}


	impl core::Vector<String> {
		pub fn as_raw_VectorOfString(&self) -> extern_send!(Self) { self.as_raw() }
		pub fn as_raw_mut_VectorOfString(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	vector_extern! { String,
		std_vectorLcv_StringG_new_const, std_vectorLcv_StringG_delete,
		std_vectorLcv_StringG_len_const, std_vectorLcv_StringG_isEmpty_const,
		std_vectorLcv_StringG_capacity_const, std_vectorLcv_StringG_shrinkToFit,
		std_vectorLcv_StringG_reserve_size_t, std_vectorLcv_StringG_remove_size_t,
		std_vectorLcv_StringG_swap_size_t_size_t, std_vectorLcv_StringG_clear,
		std_vectorLcv_StringG_get_const_size_t, std_vectorLcv_StringG_set_size_t_const_String,
		std_vectorLcv_StringG_push_const_String, std_vectorLcv_StringG_insert_size_t_const_String,
	}

	vector_non_copy_or_bool! { String }


	impl core::Vector<core::UMat> {
		pub fn as_raw_VectorOfUMat(&self) -> extern_send!(Self) { self.as_raw() }
		pub fn as_raw_mut_VectorOfUMat(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	vector_extern! { core::UMat,
		std_vectorLcv_UMatG_new_const, std_vectorLcv_UMatG_delete,
		std_vectorLcv_UMatG_len_const, std_vectorLcv_UMatG_isEmpty_const,
		std_vectorLcv_UMatG_capacity_const, std_vectorLcv_UMatG_shrinkToFit,
		std_vectorLcv_UMatG_reserve_size_t, std_vectorLcv_UMatG_remove_size_t,
		std_vectorLcv_UMatG_swap_size_t_size_t, std_vectorLcv_UMatG_clear,
		std_vectorLcv_UMatG_get_const_size_t, std_vectorLcv_UMatG_set_size_t_const_UMat,
		std_vectorLcv_UMatG_push_const_UMat, std_vectorLcv_UMatG_insert_size_t_const_UMat,
	}

	vector_non_copy_or_bool! { clone core::UMat }

	vector_boxed_ref! { core::UMat }

	vector_extern! { BoxedRef<'t, core::UMat>,
		std_vectorLcv_UMatG_new_const, std_vectorLcv_UMatG_delete,
		std_vectorLcv_UMatG_len_const, std_vectorLcv_UMatG_isEmpty_const,
		std_vectorLcv_UMatG_capacity_const, std_vectorLcv_UMatG_shrinkToFit,
		std_vectorLcv_UMatG_reserve_size_t, std_vectorLcv_UMatG_remove_size_t,
		std_vectorLcv_UMatG_swap_size_t_size_t, std_vectorLcv_UMatG_clear,
		std_vectorLcv_UMatG_get_const_size_t, std_vectorLcv_UMatG_set_size_t_const_UMat,
		std_vectorLcv_UMatG_push_const_UMat, std_vectorLcv_UMatG_insert_size_t_const_UMat,
	}


	impl core::Vector<core::Vec2d> {
		pub fn as_raw_VectorOfVec2d(&self) -> extern_send!(Self) { self.as_raw() }
		pub fn as_raw_mut_VectorOfVec2d(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	vector_extern! { core::Vec2d,
		std_vectorLcv_Vec2dG_new_const, std_vectorLcv_Vec2dG_delete,
		std_vectorLcv_Vec2dG_len_const, std_vectorLcv_Vec2dG_isEmpty_const,
		std_vectorLcv_Vec2dG_capacity_const, std_vectorLcv_Vec2dG_shrinkToFit,
		std_vectorLcv_Vec2dG_reserve_size_t, std_vectorLcv_Vec2dG_remove_size_t,
		std_vectorLcv_Vec2dG_swap_size_t_size_t, std_vectorLcv_Vec2dG_clear,
		std_vectorLcv_Vec2dG_get_const_size_t, std_vectorLcv_Vec2dG_set_size_t_const_Vec2d,
		std_vectorLcv_Vec2dG_push_const_Vec2d, std_vectorLcv_Vec2dG_insert_size_t_const_Vec2d,
	}

	vector_copy_non_bool! { core::Vec2d,
		std_vectorLcv_Vec2dG_data_const, std_vectorLcv_Vec2dG_dataMut, cv_fromSlice_const_const_Vec2dX_size_t,
		std_vectorLcv_Vec2dG_clone_const,
	}

	impl ToInputArray for core::Vector<core::Vec2d> {
		#[inline]
		fn input_array(&self) -> Result<BoxedRef<'_, core::_InputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLcv_Vec2dG_inputArray_const(self.as_raw_VectorOfVec2d(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRef::<'_, core::_InputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	input_array_ref_forward! { core::Vector<core::Vec2d> }

	impl ToOutputArray for core::Vector<core::Vec2d> {
		#[inline]
		fn output_array(&mut self) -> Result<BoxedRefMut<'_, core::_OutputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLcv_Vec2dG_outputArray(self.as_raw_mut_VectorOfVec2d(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRefMut::<'_, core::_OutputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	impl ToInputOutputArray for core::Vector<core::Vec2d> {
		#[inline]
		fn input_output_array(&mut self) -> Result<BoxedRefMut<'_, core::_InputOutputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLcv_Vec2dG_inputOutputArray(self.as_raw_mut_VectorOfVec2d(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRefMut::<'_, core::_InputOutputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	output_array_ref_forward! { core::Vector<core::Vec2d> }

	impl core::Vector<core::Vec2f> {
		pub fn as_raw_VectorOfVec2f(&self) -> extern_send!(Self) { self.as_raw() }
		pub fn as_raw_mut_VectorOfVec2f(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	vector_extern! { core::Vec2f,
		std_vectorLcv_Vec2fG_new_const, std_vectorLcv_Vec2fG_delete,
		std_vectorLcv_Vec2fG_len_const, std_vectorLcv_Vec2fG_isEmpty_const,
		std_vectorLcv_Vec2fG_capacity_const, std_vectorLcv_Vec2fG_shrinkToFit,
		std_vectorLcv_Vec2fG_reserve_size_t, std_vectorLcv_Vec2fG_remove_size_t,
		std_vectorLcv_Vec2fG_swap_size_t_size_t, std_vectorLcv_Vec2fG_clear,
		std_vectorLcv_Vec2fG_get_const_size_t, std_vectorLcv_Vec2fG_set_size_t_const_Vec2f,
		std_vectorLcv_Vec2fG_push_const_Vec2f, std_vectorLcv_Vec2fG_insert_size_t_const_Vec2f,
	}

	vector_copy_non_bool! { core::Vec2f,
		std_vectorLcv_Vec2fG_data_const, std_vectorLcv_Vec2fG_dataMut, cv_fromSlice_const_const_Vec2fX_size_t,
		std_vectorLcv_Vec2fG_clone_const,
	}

	impl ToInputArray for core::Vector<core::Vec2f> {
		#[inline]
		fn input_array(&self) -> Result<BoxedRef<'_, core::_InputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLcv_Vec2fG_inputArray_const(self.as_raw_VectorOfVec2f(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRef::<'_, core::_InputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	input_array_ref_forward! { core::Vector<core::Vec2f> }

	impl ToOutputArray for core::Vector<core::Vec2f> {
		#[inline]
		fn output_array(&mut self) -> Result<BoxedRefMut<'_, core::_OutputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLcv_Vec2fG_outputArray(self.as_raw_mut_VectorOfVec2f(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRefMut::<'_, core::_OutputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	impl ToInputOutputArray for core::Vector<core::Vec2f> {
		#[inline]
		fn input_output_array(&mut self) -> Result<BoxedRefMut<'_, core::_InputOutputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLcv_Vec2fG_inputOutputArray(self.as_raw_mut_VectorOfVec2f(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRefMut::<'_, core::_InputOutputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	output_array_ref_forward! { core::Vector<core::Vec2f> }

	impl core::Vector<core::Vec3d> {
		pub fn as_raw_VectorOfVec3d(&self) -> extern_send!(Self) { self.as_raw() }
		pub fn as_raw_mut_VectorOfVec3d(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	vector_extern! { core::Vec3d,
		std_vectorLcv_Vec3dG_new_const, std_vectorLcv_Vec3dG_delete,
		std_vectorLcv_Vec3dG_len_const, std_vectorLcv_Vec3dG_isEmpty_const,
		std_vectorLcv_Vec3dG_capacity_const, std_vectorLcv_Vec3dG_shrinkToFit,
		std_vectorLcv_Vec3dG_reserve_size_t, std_vectorLcv_Vec3dG_remove_size_t,
		std_vectorLcv_Vec3dG_swap_size_t_size_t, std_vectorLcv_Vec3dG_clear,
		std_vectorLcv_Vec3dG_get_const_size_t, std_vectorLcv_Vec3dG_set_size_t_const_Vec3d,
		std_vectorLcv_Vec3dG_push_const_Vec3d, std_vectorLcv_Vec3dG_insert_size_t_const_Vec3d,
	}

	vector_copy_non_bool! { core::Vec3d,
		std_vectorLcv_Vec3dG_data_const, std_vectorLcv_Vec3dG_dataMut, cv_fromSlice_const_const_Vec3dX_size_t,
		std_vectorLcv_Vec3dG_clone_const,
	}

	impl ToInputArray for core::Vector<core::Vec3d> {
		#[inline]
		fn input_array(&self) -> Result<BoxedRef<'_, core::_InputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLcv_Vec3dG_inputArray_const(self.as_raw_VectorOfVec3d(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRef::<'_, core::_InputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	input_array_ref_forward! { core::Vector<core::Vec3d> }

	impl ToOutputArray for core::Vector<core::Vec3d> {
		#[inline]
		fn output_array(&mut self) -> Result<BoxedRefMut<'_, core::_OutputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLcv_Vec3dG_outputArray(self.as_raw_mut_VectorOfVec3d(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRefMut::<'_, core::_OutputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	impl ToInputOutputArray for core::Vector<core::Vec3d> {
		#[inline]
		fn input_output_array(&mut self) -> Result<BoxedRefMut<'_, core::_InputOutputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLcv_Vec3dG_inputOutputArray(self.as_raw_mut_VectorOfVec3d(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRefMut::<'_, core::_InputOutputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	output_array_ref_forward! { core::Vector<core::Vec3d> }

	impl core::Vector<core::Vec3f> {
		pub fn as_raw_VectorOfVec3f(&self) -> extern_send!(Self) { self.as_raw() }
		pub fn as_raw_mut_VectorOfVec3f(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	vector_extern! { core::Vec3f,
		std_vectorLcv_Vec3fG_new_const, std_vectorLcv_Vec3fG_delete,
		std_vectorLcv_Vec3fG_len_const, std_vectorLcv_Vec3fG_isEmpty_const,
		std_vectorLcv_Vec3fG_capacity_const, std_vectorLcv_Vec3fG_shrinkToFit,
		std_vectorLcv_Vec3fG_reserve_size_t, std_vectorLcv_Vec3fG_remove_size_t,
		std_vectorLcv_Vec3fG_swap_size_t_size_t, std_vectorLcv_Vec3fG_clear,
		std_vectorLcv_Vec3fG_get_const_size_t, std_vectorLcv_Vec3fG_set_size_t_const_Vec3f,
		std_vectorLcv_Vec3fG_push_const_Vec3f, std_vectorLcv_Vec3fG_insert_size_t_const_Vec3f,
	}

	vector_copy_non_bool! { core::Vec3f,
		std_vectorLcv_Vec3fG_data_const, std_vectorLcv_Vec3fG_dataMut, cv_fromSlice_const_const_Vec3fX_size_t,
		std_vectorLcv_Vec3fG_clone_const,
	}

	impl ToInputArray for core::Vector<core::Vec3f> {
		#[inline]
		fn input_array(&self) -> Result<BoxedRef<'_, core::_InputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLcv_Vec3fG_inputArray_const(self.as_raw_VectorOfVec3f(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRef::<'_, core::_InputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	input_array_ref_forward! { core::Vector<core::Vec3f> }

	impl ToOutputArray for core::Vector<core::Vec3f> {
		#[inline]
		fn output_array(&mut self) -> Result<BoxedRefMut<'_, core::_OutputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLcv_Vec3fG_outputArray(self.as_raw_mut_VectorOfVec3f(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRefMut::<'_, core::_OutputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	impl ToInputOutputArray for core::Vector<core::Vec3f> {
		#[inline]
		fn input_output_array(&mut self) -> Result<BoxedRefMut<'_, core::_InputOutputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLcv_Vec3fG_inputOutputArray(self.as_raw_mut_VectorOfVec3f(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRefMut::<'_, core::_InputOutputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	output_array_ref_forward! { core::Vector<core::Vec3f> }

	impl core::Vector<core::Vec4f> {
		pub fn as_raw_VectorOfVec4f(&self) -> extern_send!(Self) { self.as_raw() }
		pub fn as_raw_mut_VectorOfVec4f(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	vector_extern! { core::Vec4f,
		std_vectorLcv_Vec4fG_new_const, std_vectorLcv_Vec4fG_delete,
		std_vectorLcv_Vec4fG_len_const, std_vectorLcv_Vec4fG_isEmpty_const,
		std_vectorLcv_Vec4fG_capacity_const, std_vectorLcv_Vec4fG_shrinkToFit,
		std_vectorLcv_Vec4fG_reserve_size_t, std_vectorLcv_Vec4fG_remove_size_t,
		std_vectorLcv_Vec4fG_swap_size_t_size_t, std_vectorLcv_Vec4fG_clear,
		std_vectorLcv_Vec4fG_get_const_size_t, std_vectorLcv_Vec4fG_set_size_t_const_Vec4f,
		std_vectorLcv_Vec4fG_push_const_Vec4f, std_vectorLcv_Vec4fG_insert_size_t_const_Vec4f,
	}

	vector_copy_non_bool! { core::Vec4f,
		std_vectorLcv_Vec4fG_data_const, std_vectorLcv_Vec4fG_dataMut, cv_fromSlice_const_const_Vec4fX_size_t,
		std_vectorLcv_Vec4fG_clone_const,
	}

	impl ToInputArray for core::Vector<core::Vec4f> {
		#[inline]
		fn input_array(&self) -> Result<BoxedRef<'_, core::_InputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLcv_Vec4fG_inputArray_const(self.as_raw_VectorOfVec4f(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRef::<'_, core::_InputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	input_array_ref_forward! { core::Vector<core::Vec4f> }

	impl ToOutputArray for core::Vector<core::Vec4f> {
		#[inline]
		fn output_array(&mut self) -> Result<BoxedRefMut<'_, core::_OutputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLcv_Vec4fG_outputArray(self.as_raw_mut_VectorOfVec4f(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRefMut::<'_, core::_OutputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	impl ToInputOutputArray for core::Vector<core::Vec4f> {
		#[inline]
		fn input_output_array(&mut self) -> Result<BoxedRefMut<'_, core::_InputOutputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLcv_Vec4fG_inputOutputArray(self.as_raw_mut_VectorOfVec4f(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRefMut::<'_, core::_InputOutputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	output_array_ref_forward! { core::Vector<core::Vec4f> }

	impl core::Vector<core::Vec4i> {
		pub fn as_raw_VectorOfVec4i(&self) -> extern_send!(Self) { self.as_raw() }
		pub fn as_raw_mut_VectorOfVec4i(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	vector_extern! { core::Vec4i,
		std_vectorLcv_Vec4iG_new_const, std_vectorLcv_Vec4iG_delete,
		std_vectorLcv_Vec4iG_len_const, std_vectorLcv_Vec4iG_isEmpty_const,
		std_vectorLcv_Vec4iG_capacity_const, std_vectorLcv_Vec4iG_shrinkToFit,
		std_vectorLcv_Vec4iG_reserve_size_t, std_vectorLcv_Vec4iG_remove_size_t,
		std_vectorLcv_Vec4iG_swap_size_t_size_t, std_vectorLcv_Vec4iG_clear,
		std_vectorLcv_Vec4iG_get_const_size_t, std_vectorLcv_Vec4iG_set_size_t_const_Vec4i,
		std_vectorLcv_Vec4iG_push_const_Vec4i, std_vectorLcv_Vec4iG_insert_size_t_const_Vec4i,
	}

	vector_copy_non_bool! { core::Vec4i,
		std_vectorLcv_Vec4iG_data_const, std_vectorLcv_Vec4iG_dataMut, cv_fromSlice_const_const_Vec4iX_size_t,
		std_vectorLcv_Vec4iG_clone_const,
	}

	impl ToInputArray for core::Vector<core::Vec4i> {
		#[inline]
		fn input_array(&self) -> Result<BoxedRef<'_, core::_InputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLcv_Vec4iG_inputArray_const(self.as_raw_VectorOfVec4i(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRef::<'_, core::_InputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	input_array_ref_forward! { core::Vector<core::Vec4i> }

	impl ToOutputArray for core::Vector<core::Vec4i> {
		#[inline]
		fn output_array(&mut self) -> Result<BoxedRefMut<'_, core::_OutputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLcv_Vec4iG_outputArray(self.as_raw_mut_VectorOfVec4i(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRefMut::<'_, core::_OutputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	impl ToInputOutputArray for core::Vector<core::Vec4i> {
		#[inline]
		fn input_output_array(&mut self) -> Result<BoxedRefMut<'_, core::_InputOutputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLcv_Vec4iG_inputOutputArray(self.as_raw_mut_VectorOfVec4i(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRefMut::<'_, core::_InputOutputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	output_array_ref_forward! { core::Vector<core::Vec4i> }

	impl core::Vector<core::Vec6f> {
		pub fn as_raw_VectorOfVec6f(&self) -> extern_send!(Self) { self.as_raw() }
		pub fn as_raw_mut_VectorOfVec6f(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	vector_extern! { core::Vec6f,
		std_vectorLcv_Vec6fG_new_const, std_vectorLcv_Vec6fG_delete,
		std_vectorLcv_Vec6fG_len_const, std_vectorLcv_Vec6fG_isEmpty_const,
		std_vectorLcv_Vec6fG_capacity_const, std_vectorLcv_Vec6fG_shrinkToFit,
		std_vectorLcv_Vec6fG_reserve_size_t, std_vectorLcv_Vec6fG_remove_size_t,
		std_vectorLcv_Vec6fG_swap_size_t_size_t, std_vectorLcv_Vec6fG_clear,
		std_vectorLcv_Vec6fG_get_const_size_t, std_vectorLcv_Vec6fG_set_size_t_const_Vec6f,
		std_vectorLcv_Vec6fG_push_const_Vec6f, std_vectorLcv_Vec6fG_insert_size_t_const_Vec6f,
	}

	vector_copy_non_bool! { core::Vec6f,
		std_vectorLcv_Vec6fG_data_const, std_vectorLcv_Vec6fG_dataMut, cv_fromSlice_const_const_Vec6fX_size_t,
		std_vectorLcv_Vec6fG_clone_const,
	}

	impl ToInputArray for core::Vector<core::Vec6f> {
		#[inline]
		fn input_array(&self) -> Result<BoxedRef<'_, core::_InputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLcv_Vec6fG_inputArray_const(self.as_raw_VectorOfVec6f(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRef::<'_, core::_InputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	input_array_ref_forward! { core::Vector<core::Vec6f> }

	impl ToOutputArray for core::Vector<core::Vec6f> {
		#[inline]
		fn output_array(&mut self) -> Result<BoxedRefMut<'_, core::_OutputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLcv_Vec6fG_outputArray(self.as_raw_mut_VectorOfVec6f(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRefMut::<'_, core::_OutputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	impl ToInputOutputArray for core::Vector<core::Vec6f> {
		#[inline]
		fn input_output_array(&mut self) -> Result<BoxedRefMut<'_, core::_InputOutputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLcv_Vec6fG_inputOutputArray(self.as_raw_mut_VectorOfVec6f(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRefMut::<'_, core::_InputOutputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	output_array_ref_forward! { core::Vector<core::Vec6f> }

	impl core::Vector<core::Vector<core::Point>> {
		pub fn as_raw_VectorOfVectorOfPoint(&self) -> extern_send!(Self) { self.as_raw() }
		pub fn as_raw_mut_VectorOfVectorOfPoint(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	vector_extern! { core::Vector<core::Point>,
		std_vectorLstd_vectorLcv_PointGG_new_const, std_vectorLstd_vectorLcv_PointGG_delete,
		std_vectorLstd_vectorLcv_PointGG_len_const, std_vectorLstd_vectorLcv_PointGG_isEmpty_const,
		std_vectorLstd_vectorLcv_PointGG_capacity_const, std_vectorLstd_vectorLcv_PointGG_shrinkToFit,
		std_vectorLstd_vectorLcv_PointGG_reserve_size_t, std_vectorLstd_vectorLcv_PointGG_remove_size_t,
		std_vectorLstd_vectorLcv_PointGG_swap_size_t_size_t, std_vectorLstd_vectorLcv_PointGG_clear,
		std_vectorLstd_vectorLcv_PointGG_get_const_size_t, std_vectorLstd_vectorLcv_PointGG_set_size_t_const_vectorLPointG,
		std_vectorLstd_vectorLcv_PointGG_push_const_vectorLPointG, std_vectorLstd_vectorLcv_PointGG_insert_size_t_const_vectorLPointG,
	}

	vector_non_copy_or_bool! { clone core::Vector<core::Point> }

	impl ToInputArray for core::Vector<core::Vector<core::Point>> {
		#[inline]
		fn input_array(&self) -> Result<BoxedRef<'_, core::_InputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLstd_vectorLcv_PointGG_inputArray_const(self.as_raw_VectorOfVectorOfPoint(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRef::<'_, core::_InputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	input_array_ref_forward! { core::Vector<core::Vector<core::Point>> }

	impl ToOutputArray for core::Vector<core::Vector<core::Point>> {
		#[inline]
		fn output_array(&mut self) -> Result<BoxedRefMut<'_, core::_OutputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLstd_vectorLcv_PointGG_outputArray(self.as_raw_mut_VectorOfVectorOfPoint(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRefMut::<'_, core::_OutputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	impl ToInputOutputArray for core::Vector<core::Vector<core::Point>> {
		#[inline]
		fn input_output_array(&mut self) -> Result<BoxedRefMut<'_, core::_InputOutputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLstd_vectorLcv_PointGG_inputOutputArray(self.as_raw_mut_VectorOfVectorOfPoint(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRefMut::<'_, core::_InputOutputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	output_array_ref_forward! { core::Vector<core::Vector<core::Point>> }

	impl core::Vector<core::Vector<core::Point2f>> {
		pub fn as_raw_VectorOfVectorOfPoint2f(&self) -> extern_send!(Self) { self.as_raw() }
		pub fn as_raw_mut_VectorOfVectorOfPoint2f(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	vector_extern! { core::Vector<core::Point2f>,
		std_vectorLstd_vectorLcv_Point2fGG_new_const, std_vectorLstd_vectorLcv_Point2fGG_delete,
		std_vectorLstd_vectorLcv_Point2fGG_len_const, std_vectorLstd_vectorLcv_Point2fGG_isEmpty_const,
		std_vectorLstd_vectorLcv_Point2fGG_capacity_const, std_vectorLstd_vectorLcv_Point2fGG_shrinkToFit,
		std_vectorLstd_vectorLcv_Point2fGG_reserve_size_t, std_vectorLstd_vectorLcv_Point2fGG_remove_size_t,
		std_vectorLstd_vectorLcv_Point2fGG_swap_size_t_size_t, std_vectorLstd_vectorLcv_Point2fGG_clear,
		std_vectorLstd_vectorLcv_Point2fGG_get_const_size_t, std_vectorLstd_vectorLcv_Point2fGG_set_size_t_const_vectorLPoint2fG,
		std_vectorLstd_vectorLcv_Point2fGG_push_const_vectorLPoint2fG, std_vectorLstd_vectorLcv_Point2fGG_insert_size_t_const_vectorLPoint2fG,
	}

	vector_non_copy_or_bool! { clone core::Vector<core::Point2f> }

	impl ToInputArray for core::Vector<core::Vector<core::Point2f>> {
		#[inline]
		fn input_array(&self) -> Result<BoxedRef<'_, core::_InputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLstd_vectorLcv_Point2fGG_inputArray_const(self.as_raw_VectorOfVectorOfPoint2f(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRef::<'_, core::_InputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	input_array_ref_forward! { core::Vector<core::Vector<core::Point2f>> }

	impl ToOutputArray for core::Vector<core::Vector<core::Point2f>> {
		#[inline]
		fn output_array(&mut self) -> Result<BoxedRefMut<'_, core::_OutputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLstd_vectorLcv_Point2fGG_outputArray(self.as_raw_mut_VectorOfVectorOfPoint2f(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRefMut::<'_, core::_OutputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	impl ToInputOutputArray for core::Vector<core::Vector<core::Point2f>> {
		#[inline]
		fn input_output_array(&mut self) -> Result<BoxedRefMut<'_, core::_InputOutputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLstd_vectorLcv_Point2fGG_inputOutputArray(self.as_raw_mut_VectorOfVectorOfPoint2f(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRefMut::<'_, core::_InputOutputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	output_array_ref_forward! { core::Vector<core::Vector<core::Point2f>> }

	impl core::Vector<bool> {
		pub fn as_raw_VectorOfbool(&self) -> extern_send!(Self) { self.as_raw() }
		pub fn as_raw_mut_VectorOfbool(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	vector_extern! { bool,
		std_vectorLboolG_new_const, std_vectorLboolG_delete,
		std_vectorLboolG_len_const, std_vectorLboolG_isEmpty_const,
		std_vectorLboolG_capacity_const, std_vectorLboolG_shrinkToFit,
		std_vectorLboolG_reserve_size_t, std_vectorLboolG_remove_size_t,
		std_vectorLboolG_swap_size_t_size_t, std_vectorLboolG_clear,
		std_vectorLboolG_get_const_size_t, std_vectorLboolG_set_size_t_const_bool,
		std_vectorLboolG_push_const_bool, std_vectorLboolG_insert_size_t_const_bool,
	}

	vector_non_copy_or_bool! { clone bool }

	impl ToInputArray for core::Vector<bool> {
		#[inline]
		fn input_array(&self) -> Result<BoxedRef<'_, core::_InputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLboolG_inputArray_const(self.as_raw_VectorOfbool(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRef::<'_, core::_InputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	input_array_ref_forward! { core::Vector<bool> }

	impl core::Vector<c_char> {
		pub fn as_raw_VectorOfc_char(&self) -> extern_send!(Self) { self.as_raw() }
		pub fn as_raw_mut_VectorOfc_char(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	impl core::Vector<f32> {
		pub fn as_raw_VectorOff32(&self) -> extern_send!(Self) { self.as_raw() }
		pub fn as_raw_mut_VectorOff32(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	vector_extern! { f32,
		std_vectorLfloatG_new_const, std_vectorLfloatG_delete,
		std_vectorLfloatG_len_const, std_vectorLfloatG_isEmpty_const,
		std_vectorLfloatG_capacity_const, std_vectorLfloatG_shrinkToFit,
		std_vectorLfloatG_reserve_size_t, std_vectorLfloatG_remove_size_t,
		std_vectorLfloatG_swap_size_t_size_t, std_vectorLfloatG_clear,
		std_vectorLfloatG_get_const_size_t, std_vectorLfloatG_set_size_t_const_float,
		std_vectorLfloatG_push_const_float, std_vectorLfloatG_insert_size_t_const_float,
	}

	vector_copy_non_bool! { f32,
		std_vectorLfloatG_data_const, std_vectorLfloatG_dataMut, cv_fromSlice_const_const_floatX_size_t,
		std_vectorLfloatG_clone_const,
	}

	impl ToInputArray for core::Vector<f32> {
		#[inline]
		fn input_array(&self) -> Result<BoxedRef<'_, core::_InputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLfloatG_inputArray_const(self.as_raw_VectorOff32(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRef::<'_, core::_InputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	input_array_ref_forward! { core::Vector<f32> }

	impl ToOutputArray for core::Vector<f32> {
		#[inline]
		fn output_array(&mut self) -> Result<BoxedRefMut<'_, core::_OutputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLfloatG_outputArray(self.as_raw_mut_VectorOff32(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRefMut::<'_, core::_OutputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	impl ToInputOutputArray for core::Vector<f32> {
		#[inline]
		fn input_output_array(&mut self) -> Result<BoxedRefMut<'_, core::_InputOutputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLfloatG_inputOutputArray(self.as_raw_mut_VectorOff32(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRefMut::<'_, core::_InputOutputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	output_array_ref_forward! { core::Vector<f32> }

	impl core::Vector<f64> {
		pub fn as_raw_VectorOff64(&self) -> extern_send!(Self) { self.as_raw() }
		pub fn as_raw_mut_VectorOff64(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	vector_extern! { f64,
		std_vectorLdoubleG_new_const, std_vectorLdoubleG_delete,
		std_vectorLdoubleG_len_const, std_vectorLdoubleG_isEmpty_const,
		std_vectorLdoubleG_capacity_const, std_vectorLdoubleG_shrinkToFit,
		std_vectorLdoubleG_reserve_size_t, std_vectorLdoubleG_remove_size_t,
		std_vectorLdoubleG_swap_size_t_size_t, std_vectorLdoubleG_clear,
		std_vectorLdoubleG_get_const_size_t, std_vectorLdoubleG_set_size_t_const_double,
		std_vectorLdoubleG_push_const_double, std_vectorLdoubleG_insert_size_t_const_double,
	}

	vector_copy_non_bool! { f64,
		std_vectorLdoubleG_data_const, std_vectorLdoubleG_dataMut, cv_fromSlice_const_const_doubleX_size_t,
		std_vectorLdoubleG_clone_const,
	}

	impl ToInputArray for core::Vector<f64> {
		#[inline]
		fn input_array(&self) -> Result<BoxedRef<'_, core::_InputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLdoubleG_inputArray_const(self.as_raw_VectorOff64(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRef::<'_, core::_InputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	input_array_ref_forward! { core::Vector<f64> }

	impl ToOutputArray for core::Vector<f64> {
		#[inline]
		fn output_array(&mut self) -> Result<BoxedRefMut<'_, core::_OutputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLdoubleG_outputArray(self.as_raw_mut_VectorOff64(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRefMut::<'_, core::_OutputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	impl ToInputOutputArray for core::Vector<f64> {
		#[inline]
		fn input_output_array(&mut self) -> Result<BoxedRefMut<'_, core::_InputOutputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLdoubleG_inputOutputArray(self.as_raw_mut_VectorOff64(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRefMut::<'_, core::_InputOutputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	output_array_ref_forward! { core::Vector<f64> }

	impl core::Vector<i32> {
		pub fn as_raw_VectorOfi32(&self) -> extern_send!(Self) { self.as_raw() }
		pub fn as_raw_mut_VectorOfi32(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	vector_extern! { i32,
		std_vectorLintG_new_const, std_vectorLintG_delete,
		std_vectorLintG_len_const, std_vectorLintG_isEmpty_const,
		std_vectorLintG_capacity_const, std_vectorLintG_shrinkToFit,
		std_vectorLintG_reserve_size_t, std_vectorLintG_remove_size_t,
		std_vectorLintG_swap_size_t_size_t, std_vectorLintG_clear,
		std_vectorLintG_get_const_size_t, std_vectorLintG_set_size_t_const_int,
		std_vectorLintG_push_const_int, std_vectorLintG_insert_size_t_const_int,
	}

	vector_copy_non_bool! { i32,
		std_vectorLintG_data_const, std_vectorLintG_dataMut, cv_fromSlice_const_const_intX_size_t,
		std_vectorLintG_clone_const,
	}

	impl ToInputArray for core::Vector<i32> {
		#[inline]
		fn input_array(&self) -> Result<BoxedRef<'_, core::_InputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLintG_inputArray_const(self.as_raw_VectorOfi32(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRef::<'_, core::_InputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	input_array_ref_forward! { core::Vector<i32> }

	impl ToOutputArray for core::Vector<i32> {
		#[inline]
		fn output_array(&mut self) -> Result<BoxedRefMut<'_, core::_OutputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLintG_outputArray(self.as_raw_mut_VectorOfi32(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRefMut::<'_, core::_OutputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	impl ToInputOutputArray for core::Vector<i32> {
		#[inline]
		fn input_output_array(&mut self) -> Result<BoxedRefMut<'_, core::_InputOutputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLintG_inputOutputArray(self.as_raw_mut_VectorOfi32(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRefMut::<'_, core::_InputOutputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	output_array_ref_forward! { core::Vector<i32> }

	impl core::Vector<i8> {
		pub fn as_raw_VectorOfi8(&self) -> extern_send!(Self) { self.as_raw() }
		pub fn as_raw_mut_VectorOfi8(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	vector_extern! { i8,
		std_vectorLsigned_charG_new_const, std_vectorLsigned_charG_delete,
		std_vectorLsigned_charG_len_const, std_vectorLsigned_charG_isEmpty_const,
		std_vectorLsigned_charG_capacity_const, std_vectorLsigned_charG_shrinkToFit,
		std_vectorLsigned_charG_reserve_size_t, std_vectorLsigned_charG_remove_size_t,
		std_vectorLsigned_charG_swap_size_t_size_t, std_vectorLsigned_charG_clear,
		std_vectorLsigned_charG_get_const_size_t, std_vectorLsigned_charG_set_size_t_const_signed_char,
		std_vectorLsigned_charG_push_const_signed_char, std_vectorLsigned_charG_insert_size_t_const_signed_char,
	}

	vector_copy_non_bool! { i8,
		std_vectorLsigned_charG_data_const, std_vectorLsigned_charG_dataMut, cv_fromSlice_const_const_signed_charX_size_t,
		std_vectorLsigned_charG_clone_const,
	}


	impl core::Vector<size_t> {
		pub fn as_raw_VectorOfsize_t(&self) -> extern_send!(Self) { self.as_raw() }
		pub fn as_raw_mut_VectorOfsize_t(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	vector_extern! { size_t,
		std_vectorLsize_tG_new_const, std_vectorLsize_tG_delete,
		std_vectorLsize_tG_len_const, std_vectorLsize_tG_isEmpty_const,
		std_vectorLsize_tG_capacity_const, std_vectorLsize_tG_shrinkToFit,
		std_vectorLsize_tG_reserve_size_t, std_vectorLsize_tG_remove_size_t,
		std_vectorLsize_tG_swap_size_t_size_t, std_vectorLsize_tG_clear,
		std_vectorLsize_tG_get_const_size_t, std_vectorLsize_tG_set_size_t_const_size_t,
		std_vectorLsize_tG_push_const_size_t, std_vectorLsize_tG_insert_size_t_const_size_t,
	}

	vector_copy_non_bool! { size_t,
		std_vectorLsize_tG_data_const, std_vectorLsize_tG_dataMut, cv_fromSlice_const_const_size_tX_size_t,
		std_vectorLsize_tG_clone_const,
	}


	impl core::Vector<u8> {
		pub fn as_raw_VectorOfu8(&self) -> extern_send!(Self) { self.as_raw() }
		pub fn as_raw_mut_VectorOfu8(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	vector_extern! { u8,
		std_vectorLunsigned_charG_new_const, std_vectorLunsigned_charG_delete,
		std_vectorLunsigned_charG_len_const, std_vectorLunsigned_charG_isEmpty_const,
		std_vectorLunsigned_charG_capacity_const, std_vectorLunsigned_charG_shrinkToFit,
		std_vectorLunsigned_charG_reserve_size_t, std_vectorLunsigned_charG_remove_size_t,
		std_vectorLunsigned_charG_swap_size_t_size_t, std_vectorLunsigned_charG_clear,
		std_vectorLunsigned_charG_get_const_size_t, std_vectorLunsigned_charG_set_size_t_const_unsigned_char,
		std_vectorLunsigned_charG_push_const_unsigned_char, std_vectorLunsigned_charG_insert_size_t_const_unsigned_char,
	}

	vector_copy_non_bool! { u8,
		std_vectorLunsigned_charG_data_const, std_vectorLunsigned_charG_dataMut, cv_fromSlice_const_const_unsigned_charX_size_t,
		std_vectorLunsigned_charG_clone_const,
	}

	impl ToInputArray for core::Vector<u8> {
		#[inline]
		fn input_array(&self) -> Result<BoxedRef<'_, core::_InputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLunsigned_charG_inputArray_const(self.as_raw_VectorOfu8(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRef::<'_, core::_InputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	input_array_ref_forward! { core::Vector<u8> }

	impl ToOutputArray for core::Vector<u8> {
		#[inline]
		fn output_array(&mut self) -> Result<BoxedRefMut<'_, core::_OutputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLunsigned_charG_outputArray(self.as_raw_mut_VectorOfu8(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRefMut::<'_, core::_OutputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	impl ToInputOutputArray for core::Vector<u8> {
		#[inline]
		fn input_output_array(&mut self) -> Result<BoxedRefMut<'_, core::_InputOutputArray>> {
			return_send!(via ocvrs_return);
			unsafe { sys::std_vectorLunsigned_charG_inputOutputArray(self.as_raw_mut_VectorOfu8(), ocvrs_return.as_mut_ptr()) };
			return_receive!(ocvrs_return => ret);
			let ret = ret.into_result()?;
			let ret = unsafe { BoxedRefMut::<'_, core::_InputOutputArray>::opencv_from_extern(ret) };
			Ok(ret)
		}

	}

	output_array_ref_forward! { core::Vector<u8> }

	impl core::MatOpTraitConst for types::AbstractRefMut<'static, core::MatOp> {
		#[inline] fn as_raw_MatOp(&self) -> extern_send!(Self) { self.as_raw() }
	}

	impl core::MatOpTrait for types::AbstractRefMut<'static, core::MatOp> {
		#[inline] fn as_raw_mut_MatOp(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

}
pub use core_types::*;

mod imgproc_types {
	use crate::{mod_prelude::*, core, types, sys};

	ptr_extern! { crate::imgproc::CLAHE,
		cv_PtrLcv_CLAHEG_new_null_const, cv_PtrLcv_CLAHEG_delete, cv_PtrLcv_CLAHEG_getInnerPtr_const, cv_PtrLcv_CLAHEG_getInnerPtrMut
	}

	impl core::Ptr<crate::imgproc::CLAHE> {
		#[inline] pub fn as_raw_PtrOfCLAHE(&self) -> extern_send!(Self) { self.as_raw() }
		#[inline] pub fn as_raw_mut_PtrOfCLAHE(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	impl crate::imgproc::CLAHETraitConst for core::Ptr<crate::imgproc::CLAHE> {
		#[inline] fn as_raw_CLAHE(&self) -> *const c_void { self.inner_as_raw() }
	}

	impl crate::imgproc::CLAHETrait for core::Ptr<crate::imgproc::CLAHE> {
		#[inline] fn as_raw_mut_CLAHE(&mut self) -> *mut c_void { self.inner_as_raw_mut() }
	}

	impl core::AlgorithmTraitConst for core::Ptr<crate::imgproc::CLAHE> {
		#[inline] fn as_raw_Algorithm(&self) -> *const c_void { self.inner_as_raw() }
	}

	impl core::AlgorithmTrait for core::Ptr<crate::imgproc::CLAHE> {
		#[inline] fn as_raw_mut_Algorithm(&mut self) -> *mut c_void { self.inner_as_raw_mut() }
	}

	ptr_cast_base! { core::Ptr<crate::imgproc::CLAHE>, core::Ptr<core::Algorithm>, cv_PtrLcv_CLAHEG_to_PtrOfAlgorithm }

	impl std::fmt::Debug for core::Ptr<crate::imgproc::CLAHE> {
		#[inline]
		fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
			f.debug_struct("PtrOfCLAHE")
				.finish()
		}
	}

	ptr_extern! { crate::imgproc::GeneralizedHough,
		cv_PtrLcv_GeneralizedHoughG_new_null_const, cv_PtrLcv_GeneralizedHoughG_delete, cv_PtrLcv_GeneralizedHoughG_getInnerPtr_const, cv_PtrLcv_GeneralizedHoughG_getInnerPtrMut
	}

	impl core::Ptr<crate::imgproc::GeneralizedHough> {
		#[inline] pub fn as_raw_PtrOfGeneralizedHough(&self) -> extern_send!(Self) { self.as_raw() }
		#[inline] pub fn as_raw_mut_PtrOfGeneralizedHough(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	impl crate::imgproc::GeneralizedHoughTraitConst for core::Ptr<crate::imgproc::GeneralizedHough> {
		#[inline] fn as_raw_GeneralizedHough(&self) -> *const c_void { self.inner_as_raw() }
	}

	impl crate::imgproc::GeneralizedHoughTrait for core::Ptr<crate::imgproc::GeneralizedHough> {
		#[inline] fn as_raw_mut_GeneralizedHough(&mut self) -> *mut c_void { self.inner_as_raw_mut() }
	}

	impl core::AlgorithmTraitConst for core::Ptr<crate::imgproc::GeneralizedHough> {
		#[inline] fn as_raw_Algorithm(&self) -> *const c_void { self.inner_as_raw() }
	}

	impl core::AlgorithmTrait for core::Ptr<crate::imgproc::GeneralizedHough> {
		#[inline] fn as_raw_mut_Algorithm(&mut self) -> *mut c_void { self.inner_as_raw_mut() }
	}

	ptr_cast_base! { core::Ptr<crate::imgproc::GeneralizedHough>, core::Ptr<core::Algorithm>, cv_PtrLcv_GeneralizedHoughG_to_PtrOfAlgorithm }

	impl std::fmt::Debug for core::Ptr<crate::imgproc::GeneralizedHough> {
		#[inline]
		fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
			f.debug_struct("PtrOfGeneralizedHough")
				.finish()
		}
	}

	ptr_extern! { crate::imgproc::GeneralizedHoughBallard,
		cv_PtrLcv_GeneralizedHoughBallardG_new_null_const, cv_PtrLcv_GeneralizedHoughBallardG_delete, cv_PtrLcv_GeneralizedHoughBallardG_getInnerPtr_const, cv_PtrLcv_GeneralizedHoughBallardG_getInnerPtrMut
	}

	impl core::Ptr<crate::imgproc::GeneralizedHoughBallard> {
		#[inline] pub fn as_raw_PtrOfGeneralizedHoughBallard(&self) -> extern_send!(Self) { self.as_raw() }
		#[inline] pub fn as_raw_mut_PtrOfGeneralizedHoughBallard(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	impl crate::imgproc::GeneralizedHoughBallardTraitConst for core::Ptr<crate::imgproc::GeneralizedHoughBallard> {
		#[inline] fn as_raw_GeneralizedHoughBallard(&self) -> *const c_void { self.inner_as_raw() }
	}

	impl crate::imgproc::GeneralizedHoughBallardTrait for core::Ptr<crate::imgproc::GeneralizedHoughBallard> {
		#[inline] fn as_raw_mut_GeneralizedHoughBallard(&mut self) -> *mut c_void { self.inner_as_raw_mut() }
	}

	impl core::AlgorithmTraitConst for core::Ptr<crate::imgproc::GeneralizedHoughBallard> {
		#[inline] fn as_raw_Algorithm(&self) -> *const c_void { self.inner_as_raw() }
	}

	impl core::AlgorithmTrait for core::Ptr<crate::imgproc::GeneralizedHoughBallard> {
		#[inline] fn as_raw_mut_Algorithm(&mut self) -> *mut c_void { self.inner_as_raw_mut() }
	}

	ptr_cast_base! { core::Ptr<crate::imgproc::GeneralizedHoughBallard>, core::Ptr<core::Algorithm>, cv_PtrLcv_GeneralizedHoughBallardG_to_PtrOfAlgorithm }

	impl crate::imgproc::GeneralizedHoughTraitConst for core::Ptr<crate::imgproc::GeneralizedHoughBallard> {
		#[inline] fn as_raw_GeneralizedHough(&self) -> *const c_void { self.inner_as_raw() }
	}

	impl crate::imgproc::GeneralizedHoughTrait for core::Ptr<crate::imgproc::GeneralizedHoughBallard> {
		#[inline] fn as_raw_mut_GeneralizedHough(&mut self) -> *mut c_void { self.inner_as_raw_mut() }
	}

	ptr_cast_base! { core::Ptr<crate::imgproc::GeneralizedHoughBallard>, core::Ptr<crate::imgproc::GeneralizedHough>, cv_PtrLcv_GeneralizedHoughBallardG_to_PtrOfGeneralizedHough }

	impl std::fmt::Debug for core::Ptr<crate::imgproc::GeneralizedHoughBallard> {
		#[inline]
		fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
			f.debug_struct("PtrOfGeneralizedHoughBallard")
				.finish()
		}
	}

	ptr_extern! { crate::imgproc::GeneralizedHoughGuil,
		cv_PtrLcv_GeneralizedHoughGuilG_new_null_const, cv_PtrLcv_GeneralizedHoughGuilG_delete, cv_PtrLcv_GeneralizedHoughGuilG_getInnerPtr_const, cv_PtrLcv_GeneralizedHoughGuilG_getInnerPtrMut
	}

	impl core::Ptr<crate::imgproc::GeneralizedHoughGuil> {
		#[inline] pub fn as_raw_PtrOfGeneralizedHoughGuil(&self) -> extern_send!(Self) { self.as_raw() }
		#[inline] pub fn as_raw_mut_PtrOfGeneralizedHoughGuil(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	impl crate::imgproc::GeneralizedHoughGuilTraitConst for core::Ptr<crate::imgproc::GeneralizedHoughGuil> {
		#[inline] fn as_raw_GeneralizedHoughGuil(&self) -> *const c_void { self.inner_as_raw() }
	}

	impl crate::imgproc::GeneralizedHoughGuilTrait for core::Ptr<crate::imgproc::GeneralizedHoughGuil> {
		#[inline] fn as_raw_mut_GeneralizedHoughGuil(&mut self) -> *mut c_void { self.inner_as_raw_mut() }
	}

	impl core::AlgorithmTraitConst for core::Ptr<crate::imgproc::GeneralizedHoughGuil> {
		#[inline] fn as_raw_Algorithm(&self) -> *const c_void { self.inner_as_raw() }
	}

	impl core::AlgorithmTrait for core::Ptr<crate::imgproc::GeneralizedHoughGuil> {
		#[inline] fn as_raw_mut_Algorithm(&mut self) -> *mut c_void { self.inner_as_raw_mut() }
	}

	ptr_cast_base! { core::Ptr<crate::imgproc::GeneralizedHoughGuil>, core::Ptr<core::Algorithm>, cv_PtrLcv_GeneralizedHoughGuilG_to_PtrOfAlgorithm }

	impl crate::imgproc::GeneralizedHoughTraitConst for core::Ptr<crate::imgproc::GeneralizedHoughGuil> {
		#[inline] fn as_raw_GeneralizedHough(&self) -> *const c_void { self.inner_as_raw() }
	}

	impl crate::imgproc::GeneralizedHoughTrait for core::Ptr<crate::imgproc::GeneralizedHoughGuil> {
		#[inline] fn as_raw_mut_GeneralizedHough(&mut self) -> *mut c_void { self.inner_as_raw_mut() }
	}

	ptr_cast_base! { core::Ptr<crate::imgproc::GeneralizedHoughGuil>, core::Ptr<crate::imgproc::GeneralizedHough>, cv_PtrLcv_GeneralizedHoughGuilG_to_PtrOfGeneralizedHough }

	impl std::fmt::Debug for core::Ptr<crate::imgproc::GeneralizedHoughGuil> {
		#[inline]
		fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
			f.debug_struct("PtrOfGeneralizedHoughGuil")
				.finish()
		}
	}

	ptr_extern! { crate::imgproc::LineSegmentDetector,
		cv_PtrLcv_LineSegmentDetectorG_new_null_const, cv_PtrLcv_LineSegmentDetectorG_delete, cv_PtrLcv_LineSegmentDetectorG_getInnerPtr_const, cv_PtrLcv_LineSegmentDetectorG_getInnerPtrMut
	}

	impl core::Ptr<crate::imgproc::LineSegmentDetector> {
		#[inline] pub fn as_raw_PtrOfLineSegmentDetector(&self) -> extern_send!(Self) { self.as_raw() }
		#[inline] pub fn as_raw_mut_PtrOfLineSegmentDetector(&mut self) -> extern_send!(mut Self) { self.as_raw_mut() }
	}

	impl crate::imgproc::LineSegmentDetectorTraitConst for core::Ptr<crate::imgproc::LineSegmentDetector> {
		#[inline] fn as_raw_LineSegmentDetector(&self) -> *const c_void { self.inner_as_raw() }
	}

	impl crate::imgproc::LineSegmentDetectorTrait for core::Ptr<crate::imgproc::LineSegmentDetector> {
		#[inline] fn as_raw_mut_LineSegmentDetector(&mut self) -> *mut c_void { self.inner_as_raw_mut() }
	}

	impl core::AlgorithmTraitConst for core::Ptr<crate::imgproc::LineSegmentDetector> {
		#[inline] fn as_raw_Algorithm(&self) -> *const c_void { self.inner_as_raw() }
	}

	impl core::AlgorithmTrait for core::Ptr<crate::imgproc::LineSegmentDetector> {
		#[inline] fn as_raw_mut_Algorithm(&mut self) -> *mut c_void { self.inner_as_raw_mut() }
	}

	ptr_cast_base! { core::Ptr<crate::imgproc::LineSegmentDetector>, core::Ptr<core::Algorithm>, cv_PtrLcv_LineSegmentDetectorG_to_PtrOfAlgorithm }

	impl std::fmt::Debug for core::Ptr<crate::imgproc::LineSegmentDetector> {
		#[inline]
		fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
			f.debug_struct("PtrOfLineSegmentDetector")
				.finish()
		}
	}

}
pub use imgproc_types::*;

pub use crate::manual::types::*;
